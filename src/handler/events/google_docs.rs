use core::fmt;
use std::env;

use chrono::Local;
use google_sheets4::{
    AddConditionalFormatRuleRequest, AddSheetRequest, BasicFilter, BatchUpdateSpreadsheetRequest,
    BooleanCondition, BooleanRule, CellData, CellFormat, Color, ConditionValue,
    ConditionalFormatRule, Error, GridProperties, GridRange, NumberFormat, RepeatCellRequest,
    Request, SetBasicFilterRequest, SheetProperties, Sheets, SortSpec, TextFormat, ValueRange,
};
use hyper::Client;
use log::{debug, error};
use serde::export::{fmt::Display, Formatter};
use yup_oauth2::{ServiceAccountAccess, ServiceAccountKey};

use crate::handler::{
    categorizer::{Category, CategoryProvider},
    events::{BudgetRecord, EventHandler, HandlerEvent},
};

const SS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

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
}

pub struct GoogleDocsEventHandler {
    ss_id: String,
    key: ServiceAccountKey,
}

impl GoogleDocsEventHandler {
    pub fn new() -> Self {
        let ss_id = env::var("SPREADSHEET_ID").expect("SPREADSHEET_ID must be provided");
        let creds = env::var("GSS_CREDENTIALS").expect("GSS_CREDENTIALS must be provided");
        let key = serde_json::from_str::<ServiceAccountKey>(&creds)
            .expect("GSS_CREDENTIALS must be a valid credentials JSON");

        GoogleDocsEventHandler { ss_id, key }
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

const CATEGORIES_RANGE: &str = "Categories!A1:C";

impl CategoryProvider for GoogleDocsEventHandler {
    fn categories(&self) -> Vec<Category> {
        let hub = self.hub();
        let call = hub.spreadsheets().values_get(&self.ss_id, CATEGORIES_RANGE);
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
    fn handle_event(&mut self, event: HandlerEvent) {
        match event {
            HandlerEvent::AddRecord(record) => {
                let sheet_name = record.date.format("%Y-%m").to_string();
                let sheet_id = record.date.format("%Y%m").to_string().parse().unwrap();
                let ids = self.list_sheet_ids();
                if ids.map_or(true, |ids| !ids.contains(&sheet_id)) {
                    self.add_sheet(sheet_id, &sheet_name);
                }
                let record_date = record.date;
                self.add_record(record, &sheet_name);
                if record_date != Local::today().naive_local() {
                    self.sort_sheet_data(sheet_id);
                }
            }
            HandlerEvent::UpdateRecord(_) => {}
        }
    }
}

impl GoogleDocsEventHandler {
    fn list_sheet_ids(&mut self) -> Option<Vec<i32>> {
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
                    .map(|s| s.properties.as_ref().unwrap().sheet_id.unwrap())
                    .collect();
                Some(ids)
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
        let range = format!("{}!A1", sheet_name);
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_update(data, &self.ss_id, &range)
            .value_input_option("RAW");
        if let Err(err) = call.doit() {
            error!("Error during update header: {:?}", err);
        }
    }

    fn add_record(&mut self, record: BudgetRecord, sheet_name: &str) {
        let data = ValueRange {
            range: None, //Some("A1".to_string()),
            values: Some(vec![vec![
                record.date.to_string(),
                record.amount.to_string().replace('.', ","),
                record.category,
                record.desc,
                record.user,
                record.id.to_string(),
            ]]),
            major_dimension: None, //Some("ROWS".to_string()),
        };

        let range = format!("{}!A1", sheet_name);
        let hub = self.hub();
        let call = hub
            .spreadsheets()
            .values_append(data, &self.ss_id, &range)
            .value_input_option("USER_ENTERED")
            .add_scope(SS_SCOPE);
        if let Err(err) = call.doit() {
            error!("Error during adding record: {}", err);
        }
    }

    fn sort_sheet_data(&mut self, sheet_id: i32) {
        let hub = self.hub();
        let call = hub.spreadsheets().batch_update(
            BatchUpdateSpreadsheetRequest {
                requests: Some(vec![basic_filter_request(
                    sheet_id,
                    0,
                    Columns::_Count as i32,
                )]),
                ..Default::default()
            },
            &self.ss_id,
        );
        if let Err(err) = call.doit() {
            error!("Error during adding record: {}", err);
        }
    }
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
