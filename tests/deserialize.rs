use std::path::Path;

#[test]
fn deserialize_all_reference_files() {
    let ref_dir = Path::new("tests/reference");
    if !ref_dir.exists() {
        panic!("tests/reference/ directory not found");
    }

    let mut count = 0;
    let mut total_calendars = 0;
    for entry in std::fs::read_dir(ref_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("lk") {
            eprintln!("Deserializing: {}", path.display());
            let root = legend_keeper_mcp::lk::io::read_lk_file(&path)
                .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", path.display(), e));
            eprintln!(
                "  {} — {} resources, {} calendars",
                path.file_stem().unwrap().to_str().unwrap(),
                root.resources.len(),
                root.calendars.len()
            );
            assert_eq!(root.resources.len(), root.resource_count);

            // Verify calendar content is accessible
            for cal in &root.calendars {
                assert!(!cal.name.is_empty(), "Calendar should have a name");
                assert!(!cal.months.is_empty(), "Calendar '{}' should have months", cal.name);
                eprintln!(
                    "    Calendar '{}': {} months, {} weekdays, negative_era={}",
                    cal.name,
                    cal.months.len(),
                    cal.weekdays.len(),
                    cal.negative_era.is_some()
                );
            }
            total_calendars += root.calendars.len();

            count += 1;
        }
    }
    assert!(count > 0, "No .lk files found in tests/reference/");
    assert!(total_calendars > 0, "Expected at least one calendar across reference files");
}

/// Test that a Calendar without a negativeEra field deserializes correctly.
/// This is the exact scenario that triggered the Option<Era> fix.
#[test]
fn calendar_without_negative_era() {
    let json = serde_json::json!({
        "id": "test123",
        "name": "Simple Calendar",
        "hasZeroYear": false,
        "maxMinutes": 1440,
        "months": [{"id": "m1", "name": "First", "length": 30}],
        "weekdays": [{"id": "w1", "name": "Monday"}],
        "epochWeekday": 0,
        "hoursInDay": 24,
        "minutesInHour": 60,
        "positiveEras": [{"id": "e1", "name": "CE", "abbr": "CE"}],
        "format": {"id": "f1", "year": "numeric", "month": "long", "day": "numeric", "time": "short"}
    });

    let cal: legend_keeper_mcp::lk::schema::Calendar =
        serde_json::from_value(json).expect("Calendar without negativeEra should deserialize");

    assert_eq!(cal.name, "Simple Calendar");
    assert!(cal.negative_era.is_none(), "negative_era should be None when field is absent");
    assert_eq!(cal.months.len(), 1);
    assert_eq!(cal.positive_eras.len(), 1);
}

/// Test that a Calendar with an explicit negativeEra deserializes and round-trips correctly.
#[test]
fn calendar_with_negative_era_roundtrips() {
    let json = serde_json::json!({
        "id": "test456",
        "name": "Full Calendar",
        "hasZeroYear": false,
        "maxMinutes": 1440,
        "months": [{"id": "m1", "name": "First", "length": 30}],
        "weekdays": [{"id": "w1", "name": "Monday"}],
        "epochWeekday": 0,
        "hoursInDay": 24,
        "minutesInHour": 60,
        "negativeEra": {"id": "ne1", "name": "BCE", "abbr": "BCE"},
        "positiveEras": [{"id": "e1", "name": "CE", "abbr": "CE"}],
        "format": {"id": "f1", "year": "numeric", "month": "long", "day": "numeric", "time": "short"}
    });

    let cal: legend_keeper_mcp::lk::schema::Calendar =
        serde_json::from_value(json).expect("Calendar with negativeEra should deserialize");

    assert!(cal.negative_era.is_some(), "negative_era should be Some when field is present");
    assert_eq!(cal.negative_era.as_ref().unwrap().name, "BCE");

    // Round-trip: serialize and deserialize again
    let serialized = serde_json::to_value(&cal).unwrap();

    // Verify negativeEra is present in serialized output (not skipped)
    assert!(serialized.get("negativeEra").is_some(), "negativeEra should be serialized when Some");
    assert_eq!(serialized["negativeEra"]["name"], "BCE");

    let cal2: legend_keeper_mcp::lk::schema::Calendar =
        serde_json::from_value(serialized).expect("Round-tripped calendar should deserialize");
    assert_eq!(cal2.negative_era.as_ref().unwrap().name, "BCE");
}

/// Test that None negative_era is omitted (not serialized as null).
#[test]
fn calendar_none_negative_era_omits_field() {
    let json = serde_json::json!({
        "id": "test789",
        "name": "No Era Cal",
        "hasZeroYear": false,
        "maxMinutes": 1440,
        "months": [],
        "weekdays": [],
        "epochWeekday": 0,
        "hoursInDay": 24,
        "minutesInHour": 60,
        "format": {"id": "f1", "year": "numeric", "month": "long", "day": "numeric", "time": "short"}
    });

    let cal: legend_keeper_mcp::lk::schema::Calendar =
        serde_json::from_value(json).unwrap();

    let serialized = serde_json::to_value(&cal).unwrap();
    assert!(
        serialized.get("negativeEra").is_none(),
        "negativeEra should be omitted when None, got: {}",
        serde_json::to_string_pretty(&serialized).unwrap()
    );
}
