// SPDX-License-Identifier: MIT OR Apache-2.0
use super::annotate::{filter_annotations, project_annotations};
use super::types::{RawAnnotation, Rect, ScreenshotOptions};

#[test]
fn filters_annotations_to_target_overlap() {
    let annotations = vec![
        RawAnnotation {
            ref_id: "e1".to_string(),
            number: 1,
            role: "button".to_string(),
            name: Some("Inside".to_string()),
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 50.0,
                height: 20.0,
            },
        },
        RawAnnotation {
            ref_id: "e2".to_string(),
            number: 2,
            role: "button".to_string(),
            name: Some("Outside".to_string()),
            rect: Rect {
                x: 200.0,
                y: 200.0,
                width: 40.0,
                height: 20.0,
            },
        },
    ];

    let target = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };

    let filtered = filter_annotations(annotations, Some(&target));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].ref_id, "e1");
}

#[test]
fn projects_selector_annotations_relative_to_target() {
    let annotations = vec![RawAnnotation {
        ref_id: "e1".to_string(),
        number: 1,
        role: "button".to_string(),
        name: Some("Inside".to_string()),
        rect: Rect {
            x: 25.0,
            y: 35.0,
            width: 40.0,
            height: 20.0,
        },
    }];

    let target = Rect {
        x: 10.0,
        y: 15.0,
        width: 100.0,
        height: 100.0,
    };

    let projected = project_annotations(&annotations, Some(&target), None);
    assert_eq!(projected[0].box_.x, 15);
    assert_eq!(projected[0].box_.y, 20);
}

#[test]
fn screenshot_options_accept_full_page_quality_element() {
    let opts = ScreenshotOptions {
        full_page: true,
        quality: Some(72),
        format: "jpeg".into(),
        ..ScreenshotOptions::default()
    };
    assert!(opts.full_page);
    assert_eq!(opts.quality, Some(72));
    assert_eq!(opts.format, "jpeg");
}

#[test]
fn projects_full_page_annotations_to_document_space() {
    let annotations = vec![RawAnnotation {
        ref_id: "e1".to_string(),
        number: 1,
        role: "button".to_string(),
        name: Some("Bottom".to_string()),
        rect: Rect {
            x: 5.0,
            y: 12.0,
            width: 40.0,
            height: 20.0,
        },
    }];

    let projected = project_annotations(&annotations, None, Some((10.0, 1000.0)));
    assert_eq!(projected[0].box_.x, 15);
    assert_eq!(projected[0].box_.y, 1012);
}
