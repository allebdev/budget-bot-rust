use core::fmt;
use std::collections::HashMap;
use std::env;

use chrono::{Local, NaiveDate};
use google_sheets4::{
    AddConditionalFormatRuleRequest, AddSheetRequest, BasicFilter, BatchUpdateSpreadsheetRequest,
    BooleanCondition, BooleanRule, CellData, CellFormat, ClearValuesRequest, Color, ConditionValue,
    ConditionalFormatRule, Error, GridCoordinate, GridProperties, GridRange, NumberFormat,
    PivotGroup, PivotTable, PivotValue, RepeatCellRequest, Request, RowData, SetBasicFilterRequest,
    SheetProperties, Sheets, SortSpec, TextFormat, UpdateCellsRequest, ValueRange,
};
use hyper::Client;
use log::{debug, error, warn};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::export::{fmt::Display, Formatter};
use yup_oauth2::{ServiceAccountAccess, ServiceAccountKey};

use crate::handler::{
    categorizer::{Category, CategoryProvider},
    events::{BudgetRecord, EventHandler, HandlerEvent},
};

const SS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

#[allow(dead_code)]
enum SortOrder {
    Ascending,
    Descending,
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SortOrder::Ascending => write!(f, "ASCENDING"),
            SortOrder::Descending => write!(f, "DESCENDING"),
        }
    }
}

#[allow(dead_code)]
enum Columns {
    Date,
    Amount,
    Category,
    Description,
    User,
    MessageId,
    _Count,
    _PivotTable,
}

impl Columns {
    fn name(self) -> String {
        match self as i32 {
            0 => String::from("A"),
            1 => String::from("B"),
            2 => String::from("C"),
            3 => String::from("D"),
            4 => String::from("E"),
            5 => String::from("F"),
            6 => String::from("G"),
            7 => String::from("H"),
            8 => String::from("I"),
            9 => String::from("J"),
            _ => unreachable!(),
        }
    }
}

trait NaiveDateExt {
    fn get_sheet_id(&self) -> i32;
}

impl NaiveDateExt for NaiveDate {
    fn get_sheet_id(&self) -> i32 {
        self.format("%Y%m").to_string().parse().unwrap()
    }
}

trait BudgetRecordExt {
    fn to_value_range(&self, range: Option<&str>, major_dimension: Option<&str>) -> ValueRange;
}

impl BudgetRecordExt for BudgetRecord {
    fn to_value_range(&self, range: Option<&str>, major_dimension: Option<&str>) -> ValueRange {
        ValueRange {
            range: range.map(|s| s.to_owned()),
            values: Some(vec![vec![
                self.date.to_string(),
                self.amount.to_string().replace('.', ","),
                self.category.to_owned(),
                self.desc.to_owned(),
                self.user.to_owned(),
                self.id.to_string(),
            ]]),
            major_dimension: major_dimension.map(|s| s.to_owned()),
        }
    }
}

pub struct GoogleDocsEventHandler {
    categories_sheet_name: String,
    data_sheet_name_format: String,
    key: ServiceAccountKey,
    ss_id: String,
}

impl GoogleDocsEventHandler {
    pub fn new() -> Self {
        let ss_id = env::var("GSS_SPREADSHEET_ID").expect("GSS_SPREADSHEET_ID must be provided");
        let creds = env::var("GSS_CREDENTIALS").expect("GSS_CREDENTIALS must be provided");
        let data_sheet_name_format =
            env::var("GSS_DATA_SHEET_NAME_FORMAT").unwrap_or("%Y-%m".to_owned());
        let categories_sheet_name =
            env::var("GSS_CATEGORIES_SHEET_NAME").unwrap_or("Categories".to_owned());
        let key = serde_json::from_str::<ServiceAccountKey>(&creds)
            .expect("GSS_CREDENTIALS must be a valid credentials JSON");

        GoogleDocsEventHandler {
            categories_sheet_name,
            ss_id,
            key,
            data_sheet_name_format,
        }
    }

    fn auth(&self) -> ServiceAccountAccess<Client> {
        ServiceAccountAccess::new(
            self.key.clone(),
            hyper::Client::with_connector(hyper::net::HttpsConnector::new(
                hyper_rustls::TlsClient::new(),
            )),
        )
    }

    // todo: probably not very efficient way to avoid losing Sync for this handler
    fn hub(&self) -> Sheets<Client, ServiceAccountAccess<Client>> {
        Sheets::new(
            hyper::Client::with_connector(hyper::net::HttpsConnector::new(
                hyper_rustls::TlsClient::new(),
            )),
            self.auth(),
        )
    }
}

impl CategoryProvider for GoogleDocsEventHandler {
    fn categories(&self) -> Vec<Category> {
        let hub = self.hub();
        let range = gss_range(&self.categories_sheet_name, "A1:C");
        let call = hub.spreadsheets().values_get(&self.ss_id, &range);
        if let Some(data) = call.doit().expect("Can not fetch categories").1.values {
            data.iter()
                .map(|c| {
                    let name = c.get(1).expect("Missing name for category").to_owned();
                    let priority = c
                        .get(0)
                        .expect("Missing priority for category")
                        .parse()
                        .expect("Priority must be a number");
                    let lexemes = c.get(2).map(|s| s.to_owned()).unwrap_or(String::new());
                    Category::new(name, priority, lexemes.as_str().into())
                })
                .collect()
        } else {
            vec![]
        }
    }
}

impl EventHandler for GoogleDocsEventHandler {
    fn handle_event(&mut self, event: HandlerEvent) -> Result<(), String> {
        match event {
            HandlerEvent::AddRecord(record) => {
                let sheet_id = record.date.get_sheet_id();
                let sheet_name = self.get_or_create_sheet_by_date(&record.date);
                self.add_record(&record, &sheet_name);
                if record.date != Local::today().naive_local() {
                    self.sort_sheets_data(&[sheet_id]);
                }
                Ok(())
            }
            HandlerEvent::UpdateRecord(record) => {
                let new_sheet_name = self.get_or_create_sheet_by_date(&record.date);
                if let Some(range) = self.find_record_range(&record, &new_sheet_name) {
                    // The same month as previous version has
                    self.update_record(&record, &range);
                    self.sort_sheets_data(&[record.date.get_sheet_id()]); // TODO: check previous date in table
                    return Ok(());
                }

                let id = record.create_date.get_sheet_id();
                let sheet_ids = last_sheet_ids(id, 12);
                if let Some(range) =
                    self.get_existing_sheet_names(sheet_ids)
                        .and_then(|sheet_names| {
                            debug!(
                                "Search record #{} in next sheets: {:?}",
                                record.id, sheet_names
                            );
                            self.find_record_range_on_sheets(sheet_names, &record)
                        })
                {
                    debug!("Record #{} found in range {}", record.id, range);
                    self.add_record(&record, &new_sheet_name);
                    self.clear_record(&record, &range);
                    self.sort_sheets_data(&[
                        record.date.get_sheet_id(),
                        record.create_date.get_sheet_id(),
                    ]); // TODO: check previous date in table
                    return Ok(());
                } else {
                    warn!("Record with id={} was not found", record.id);
                    Err(format!("Record with id={} was not found", record.id))
                }
            }
        }
    }
}

impl GoogleDocsEventHandler {
    fn get_or_create_sheet_by_date(&mut self, date: &NaiveDate) -> String {
        let sheet_id = date.get_sheet_id();
        let mut sheet_name = self.get_sheet_name(sheet_id);
        if sheet_name.is_none() {
            let name = date.format(&self.data_sheet_name_format).to_string();
            self.add_sheet(sheet_id, &name);
            sheet_name.replace(name);
        }
        sheet_name.unwrap()
    }

    fn get_sheet_name(&mut self, sheet_id: i32) -> Option<String> {
        self.list_sheets_names()
            .and_then(|info| info.get(&sheet_id).cloned())
    }

    fn get_existing_sheet_names(&mut self, sheet_ids: Vec<i32>) -> Option<Vec<String>> {
        let sheet_names = self.list_sheets_names()?;
        Some(
            sheet_names
                .iter()
                .filter(|(id, _)| sheet_ids.contains(id))
                .map(|(_, name)| name.to_owned())
                .collect(),
        )
    }

    fn list_sheets_names(&mut self) -> Option<HashMap<i32, String>> {
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .get(&self.ss_id)
            .param("fields", "sheets.properties");
        match call.doit() {
            Ok(response) => {
                let sheets = response.1.sheets?;
                let ids = sheets
                    .iter()
                    .map(|s| {
                        let props = s.properties.as_ref().unwrap();
                        let sheet_id = props.sheet_id.unwrap();
                        let sheet_title =
                            props.title.clone().unwrap_or_else(|| sheet_id.to_string());
                        (sheet_id, sheet_title)
                    })
                    .collect();
                let result = HashMap::from(ids);
                Some(result)
            }
            Err(..) => None,
        }
    }

    fn add_sheet(&mut self, sheet_id: i32, sheet_name: &str) {
        let hub = self.hub();
        let call = hub.spreadsheets().batch_update(
            BatchUpdateSpreadsheetRequest {
                requests: Some(vec![
                    add_sheet_request(sheet_id, sheet_name),
                    hide_the_same_date_conditional_format_request(sheet_id),
                    number_format_request(
                        sheet_id,
                        Columns::Date as i32,
                        NumberFormat {
                            pattern: Some("dd, ddd".to_string()),
                            type_: Some("DATE".to_string()),
                        },
                    ),
                    number_format_request(
                        sheet_id,
                        Columns::Amount as i32,
                        NumberFormat {
                            pattern: Some("#,##0.00".to_string()),
                            type_: Some("NUMBER".to_string()),
                        },
                    ),
                    number_format_request(
                        sheet_id,
                        Columns::MessageId as i32,
                        NumberFormat {
                            pattern: None,
                            type_: Some("TEXT".to_string()),
                        },
                    ),
                    basic_filter_request(sheet_id, 0, Columns::_Count as i32),
                    add_pivot_table_request(sheet_id),
                ]),
                ..Default::default()
            },
            &self.ss_id,
        );
        match call.doit() {
            Err(err) => {
                match err {
                    Error::Failure(response) => {
                        error!("GSS BadRequest: {:?}", response);
                        // todo: check if sheet already exists
                    }
                    err => error!("Error during creation month sheet: {}", err),
                }
            }
            Ok(..) => {
                debug!("New sheet created: {}", sheet_name);
                self.update_header(sheet_name);
            }
        }
    }

    fn update_header(&mut self, sheet_name: &str) {
        let data = ValueRange {
            values: Some(vec![vec![
                "Date".to_string(),
                "Amount".to_string(),
                "Category".to_string(),
                "Description".to_string(),
                "User".to_string(),
                "Message Id".to_string(),
            ]]),
            ..Default::default()
        };
        let range = gss_range(sheet_name, "A1");
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_update(data, &self.ss_id, &range)
            .value_input_option("RAW");
        if let Err(err) = call.doit() {
            error!("Error during update header: {:?}", err);
        }
    }

    fn add_record(&mut self, record: &BudgetRecord, sheet_name: &str) {
        let data = record.to_value_range(None, None);
        let range = gss_range(sheet_name, "A1");
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_append(data, &self.ss_id, &range)
            .value_input_option("USER_ENTERED")
            .add_scope(SS_SCOPE);
        if let Err(err) = call.doit() {
            error!("Error during adding record with id={}: {}", record.id, err);
        }
    }

    fn update_record(&mut self, record: &BudgetRecord, range: &str) {
        let data = record.to_value_range(Some(range), None);
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_update(data, &self.ss_id, range)
            .value_input_option("USER_ENTERED")
            .add_scope(SS_SCOPE);
        if let Err(err) = call.doit() {
            error!(
                "Error during updating record with id={}: {}",
                record.id, err
            );
        }
    }

    fn clear_record(&mut self, record: &BudgetRecord, range: &str) {
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_clear(ClearValuesRequest::default(), &self.ss_id, range)
            .add_scope(SS_SCOPE);
        if let Err(err) = call.doit() {
            error!(
                "Error during clearing record with id={}: {}",
                record.id, err
            );
        }
    }

    fn find_record_range(&mut self, record: &BudgetRecord, sheet_name: &str) -> Option<String> {
        let range = gss_range(
            sheet_name,
            &format!("{col}:{col}", col = Columns::MessageId.name()),
        );
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_get(&self.ss_id, &range)
            .major_dimension("COLUMNS")
            .value_render_option("FORMATTED_VALUE")
            .add_scope(SS_SCOPE);
        let result = call.doit();
        match result {
            Ok((_, value_range)) => {
                let id = record.id.to_string();
                let row_index = value_range.values.and_then(|rows| {
                    rows.first()
                        .and_then(|cols| cols.iter().position(|v| *v == id))
                });
                row_index.map(|idx| gss_range(sheet_name, &format!("A{row}:{row}", row = idx + 1)))
            }
            Err(_) => {
                error!("Record #{} is not found", record.id);
                None
            }
        }
    }

    fn find_record_range_on_sheets(
        &mut self,
        sheet_names: Vec<String>,
        record: &BudgetRecord,
    ) -> Option<String> {
        let hub = self.hub();
        let mut call = hub
            .spreadsheets()
            .values_batch_get(&self.ss_id)
            .major_dimension("COLUMNS")
            .value_render_option("FORMATTED_VALUE")
            .add_scope(SS_SCOPE);
        for sheet_name in sheet_names {
            call = call.add_ranges(&gss_range(
                &sheet_name,
                &format!("{col}:{col}", col = Columns::MessageId.name()),
            ));
        }
        match call.doit() {
            Ok((_, data)) => {
                let id = record.id.to_string();
                data.value_ranges.and_then(|ranges| {
                    let result = ranges.iter().find_map(|range| {
                        range
                            .range
                            .as_ref()
                            .zip(range.values.as_ref().and_then(|rows| {
                                rows.first()
                                    .and_then(|cols| cols.iter().position(|v| *v == id))
                            }))
                    });
                    result.and_then(|(range, index)| {
                        range.split("!").next().map(|sheet_name| {
                            gss_range(sheet_name, &format!("A{row}:{row}", row = index + 1))
                        })
                    })
                })
            }
            Err(_) => {
                error!("Record #{} is not found", record.id);
                None
            }
        }
    }

    fn sort_sheets_data(&mut self, sheet_ids: &[i32]) {
        let filter_requests: Vec<Request> = sheet_ids
            .iter()
            .map(|&sheet_id| basic_filter_request(sheet_id, 0, Columns::_Count as i32))
            .collect();
        let hub = self.hub();
        let call = hub.spreadsheets().batch_update(
            BatchUpdateSpreadsheetRequest {
                requests: Some(filter_requests),
                ..Default::default()
            },
            &self.ss_id,
        );
        if let Err(err) = call.doit() {
            error!(
                "Error during setting filter for sheets with ids {:?}: {}",
                sheet_ids, err
            );
        }
    }
}

#[inline]
fn gss_range(sheet_name: &str, a1range: &str) -> String {
    format!(
        "{}!{}",
        utf8_percent_encode(sheet_name, NON_ALPHANUMERIC),
        a1range
    )
}

#[inline]
fn add_sheet_request(sheet_id: i32, sheet_name: &str) -> Request {
    Request {
        add_sheet: Some(AddSheetRequest {
            properties: Some(SheetProperties {
                title: Some(sheet_name.to_string()),
                sheet_id: Some(sheet_id),
                grid_properties: Some(GridProperties {
                    frozen_row_count: Some(1),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

#[inline]
fn number_format_request(sheet_id: i32, column: i32, number_format: NumberFormat) -> Request {
    Request {
        repeat_cell: Some(RepeatCellRequest {
            cell: Some(CellData {
                user_entered_format: Some(CellFormat {
                    number_format: Some(number_format),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            fields: Some("userEnteredFormat.numberFormat".to_string()),
            range: Some(GridRange {
                end_column_index: Some(column + 1),
                sheet_id: Some(sheet_id),
                start_column_index: Some(column),
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

#[inline]
fn basic_filter_request(sheet_id: i32, start_column: i32, end_column: i32) -> Request {
    Request {
        set_basic_filter: Some(SetBasicFilterRequest {
            filter: Some(BasicFilter {
                range: Some(GridRange {
                    sheet_id: Some(sheet_id),
                    start_column_index: Some(start_column),
                    end_column_index: Some(end_column),
                    ..Default::default()
                }),
                sort_specs: Some(vec![
                    SortSpec {
                        dimension_index: Some(0),
                        sort_order: Some(SortOrder::Ascending.to_string()),
                        ..Default::default()
                    },
                    SortSpec {
                        dimension_index: Some(5),
                        sort_order: Some(SortOrder::Ascending.to_string()),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

#[inline]
fn hide_the_same_date_conditional_format_request(sheet_id: i32) -> Request {
    Request {
        add_conditional_format_rule: Some(AddConditionalFormatRuleRequest {
            index: Some(0),
            rule: Some(ConditionalFormatRule {
                ranges: Some(vec![GridRange {
                    end_column_index: Some(Columns::Date as i32 + 1),
                    sheet_id: Some(sheet_id),
                    start_column_index: Some(Columns::Date as i32),
                    ..Default::default()
                }]),
                boolean_rule: Some(BooleanRule {
                    condition: Some(BooleanCondition {
                        type_: Some("NUMBER_EQ".to_string()),
                        values: Some(vec![ConditionValue {
                            relative_date: None,
                            user_entered_value: Some(
                                "=INDIRECT(\"R\"&ROW()-1&\"C\"&COL();FALSE)".to_string(),
                            ),
                        }]),
                    }),
                    format: Some(CellFormat {
                        text_format: Some(TextFormat {
                            foreground_color: Some(Color {
                                blue: Some(1.0),
                                green: Some(1.0),
                                red: Some(1.0),
                                alpha: None,
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                }),
                gradient_rule: None,
            }),
        }),
        ..Default::default()
    }
}

#[inline]
fn add_pivot_table_request(sheet_id: i32) -> Request {
    Request {
        update_cells: Some(UpdateCellsRequest {
            start: Some(GridCoordinate {
                sheet_id: Some(sheet_id),
                column_index: Some(Columns::_PivotTable as i32),
                row_index: Some(0),
            }),
            fields: Some("pivotTable".to_string()),
            rows: Some(vec![RowData {
                values: Some(vec![CellData {
                    pivot_table: Some(PivotTable {
                        // todo: add filterSpecs when new version of google-sheets4 is released
                        source: Some(GridRange {
                            sheet_id: Some(sheet_id),
                            start_column_index: Some(0),
                            end_column_index: Some(Columns::_Count as i32),
                            ..Default::default()
                        }),
                        values: Some(vec![PivotValue {
                            summarize_function: Some("SUM".to_string()),
                            source_column_offset: Some(Columns::Amount as i32),
                            ..Default::default()
                        }]),
                        rows: Some(vec![PivotGroup {
                            source_column_offset: Some(Columns::Category as i32),
                            show_totals: Some(true),
                            sort_order: Some(SortOrder::Ascending.to_string()),
                            ..Default::default()
                        }]),
                        columns: Some(vec![PivotGroup {
                            source_column_offset: Some(Columns::User as i32),
                            show_totals: Some(true),
                            sort_order: Some(SortOrder::Ascending.to_string()),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }),
                    ..Default::default()
                }]),
            }]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn last_sheet_ids(id: i32, count: usize) -> Vec<i32> {
    (0..count - 1).fold(vec![id], |mut v, _| {
        let id = v.last().unwrap();
        let prev = if (id - 1) % 100 == 0 { id - 89 } else { id - 1 };
        v.push(prev);
        v
    })
}

#[cfg(test)]
mod tests {
    use crate::handler::events::google_docs::last_sheet_ids;

    #[test]
    fn last_4_sheet_ids() {
        assert_eq!(
            last_sheet_ids(202102, 4),
            vec![202102, 202101, 202012, 202011]
        )
    }
}
