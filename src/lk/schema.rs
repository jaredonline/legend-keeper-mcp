use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LkRoot {
    pub version: u32,
    pub export_id: String,
    pub exported_at: String,
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub calendars: Vec<Calendar>,
    pub resource_count: usize,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub pos: String,
    pub created_by: String,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub show_property_bar: bool,
    pub icon_color: Option<String>,
    pub icon_glyph: Option<String>,
    pub icon_shape: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub documents: Vec<Document>,
    #[serde(default)]
    pub properties: Vec<Property>,
    pub banner: Banner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    pub locator_id: String,
    pub pos: String,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub is_first: bool,
    pub is_full_width: Option<bool>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub transforms: Vec<Value>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub presentation: Option<Presentation>,
    pub content: Option<Value>,
    pub map: Option<MapData>,
    pub calendar_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub id: String,
    pub pos: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub title: String,
    #[serde(default)]
    pub is_hidden: Option<bool>,
    #[serde(default)]
    pub is_title_hidden: Option<bool>,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Banner {
    pub enabled: bool,
    pub url: String,
    pub y_position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    #[serde(rename = "locatorId")]
    pub locator_id: String,
    #[serde(rename = "mapId")]
    pub map_id: String,
    pub min_x: i64,
    pub max_x: i64,
    pub min_y: i64,
    pub max_y: i64,
    pub max_zoom: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub has_zero_year: bool,
    pub max_minutes: i64,
    #[serde(default)]
    pub months: Vec<Month>,
    #[serde(default)]
    pub leap_days: Vec<Value>,
    #[serde(default)]
    pub weekdays: Vec<Weekday>,
    pub epoch_weekday: u32,
    #[serde(default)]
    pub week_resets_each_month: bool,
    pub hours_in_day: u32,
    pub minutes_in_hour: u32,
    pub negative_era: Era,
    #[serde(default)]
    pub positive_eras: Vec<Era>,
    #[serde(default)]
    pub moons: Vec<Value>,
    pub format: CalendarFormat,
    #[serde(default)]
    pub half_clock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Month {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_intercalary: bool,
    pub length: u32,
    #[serde(default)]
    pub interval: Option<u32>,
    #[serde(default)]
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weekday {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Era {
    pub id: String,
    pub name: String,
    pub abbr: String,
    #[serde(default)]
    pub hide_abbr: bool,
    pub starts_at: Option<Value>,
    pub reset_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarFormat {
    pub id: String,
    pub year: String,
    pub month: String,
    pub day: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapFeature {
    pub id: String,
    pub name: String,
    pub pos: [f64; 2],
    #[serde(rename = "type")]
    pub feature_type: Option<String>,
    pub rank: Option<String>,
    #[serde(default)]
    pub is_synced: bool,
    // Pin fields
    pub uri: Option<String>,
    pub icon_glyph: Option<String>,
    pub icon_color: Option<String>,
    pub icon_shape: Option<String>,
    // Region fields
    pub polygon: Option<Vec<[f64; 2]>>,
    pub fill_opacity: Option<f64>,
    pub fill_visibility: Option<String>,
    pub label_visibility: Option<String>,
    pub border_style: Option<String>,
    pub fill_style: Option<String>,
    // Label fields
    pub label_size: Option<String>,
    pub font_family: Option<String>,
    pub color_a: Option<String>,
    pub color_b: Option<String>,
    pub label_style: Option<Value>,
    // Path fields
    pub polyline: Option<Vec<[f64; 2]>>,
    pub color: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_style: Option<String>,
    pub stroke_opacity: Option<f64>,
    pub curviness: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapContent {
    #[serde(default)]
    pub pins: Vec<MapFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineContent {
    #[serde(default)]
    pub lanes: Vec<Lane>,
    #[serde(default)]
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lane {
    pub id: String,
    pub name: String,
    pub pos: String,
    pub size: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: String,
    pub lane_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub pos: String,
    pub detail: Option<Value>,
    pub start: i64,
    pub end: Option<i64>,
    pub name: String,
    pub icon_glyph: Option<String>,
    pub color: Option<String>,
    pub image_url: Option<String>,
    pub image_fit: Option<String>,
    pub opacity: Option<f64>,
    #[serde(default)]
    pub is_synced: bool,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub uri: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub resource_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    pub document_type: String,
    pub calibration: Option<Calibration>,
    pub default_mode: Option<String>,
    pub disallowed_modes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub real_units_per_map_unit: f64,
    pub unit: String,
    pub calibration_distance: f64,
    pub calibration_map_distance: f64,
}
