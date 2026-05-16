use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use csv::Reader;
use plotters::prelude::*;

use crate::app::output::ExperimentRecord;

const CHART_SIZE: (u32, u32) = (1400, 600);

#[derive(Debug, Clone, PartialEq)]
struct MetricPoint {
    x: f64,
    value: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct LabeledMetricPoint {
    label: String,
    value: f64,
}

pub fn write_plots(
    users_input: &Path,
    cw_input: &Path,
    mixed_input: &Path,
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let users_records = read_records(users_input)?;
    let cw_records = read_records(cw_input)?;
    let mixed_records = read_records(mixed_input)?;

    let users_delay = aggregate_numeric_metric(&users_records, |record| {
        Some((record.total_users as f64, record.average_delay_slots))
    });
    let users_throughput = aggregate_numeric_metric(&users_records, |record| {
        Some((record.total_users as f64, record.throughput_bits_per_slot))
    });
    let cw_delay = aggregate_numeric_metric(&cw_records, |record| {
        record
            .cw_min
            .map(|cw_min| (cw_min as f64, record.average_delay_slots))
    });
    let cw_throughput = aggregate_numeric_metric(&cw_records, |record| {
        record
            .cw_min
            .map(|cw_min| (cw_min as f64, record.throughput_bits_per_slot))
    });
    let mixed_delay = aggregate_labeled_metric(&mixed_records, |record| {
        (record.class_name.clone(), record.average_delay_slots)
    });
    let mixed_throughput = aggregate_labeled_metric(&mixed_records, |record| {
        (record.class_name.clone(), record.throughput_bits_per_slot)
    });

    let users_output = output_dir.join("users.png");
    let cw_output = output_dir.join("cw.png");
    let mixed_output = output_dir.join("mixed.png");

    draw_dual_line_chart(
        &users_output,
        "Users Sweep",
        "Users",
        "Average Delay (slots)",
        "Throughput (bits/slot)",
        &users_delay,
        &users_throughput,
    )?;
    draw_dual_line_chart(
        &cw_output,
        "CWmin Sweep",
        "CWmin",
        "Average Delay (slots)",
        "Throughput (bits/slot)",
        &cw_delay,
        &cw_throughput,
    )?;
    draw_dual_bar_chart(
        &mixed_output,
        "Mixed Class Comparison",
        "Average Delay (slots)",
        "Throughput (bits/slot)",
        &mixed_delay,
        &mixed_throughput,
    )?;

    Ok(vec![users_output, cw_output, mixed_output])
}

fn read_records(path: &Path) -> Result<Vec<ExperimentRecord>> {
    let mut reader =
        Reader::from_path(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut records = Vec::new();

    for record in reader.deserialize() {
        records.push(record.with_context(|| format!("failed to parse {}", path.display()))?);
    }

    ensure!(
        !records.is_empty(),
        "{} did not contain any records",
        path.display()
    );

    Ok(records)
}

fn aggregate_numeric_metric(
    records: &[ExperimentRecord],
    value_fn: impl Fn(&ExperimentRecord) -> Option<(f64, f64)>,
) -> Vec<MetricPoint> {
    let mut grouped: BTreeMap<i64, (f64, usize)> = BTreeMap::new();

    for record in records {
        if let Some((x, value)) = value_fn(record) {
            let bucket = grouped
                .entry((x * 1000.0).round() as i64)
                .or_insert((0.0, 0));
            bucket.0 += value;
            bucket.1 += 1;
        }
    }

    grouped
        .into_iter()
        .map(|(x, (total, count))| MetricPoint {
            x: x as f64 / 1000.0,
            value: total / count as f64,
        })
        .collect()
}

fn aggregate_labeled_metric(
    records: &[ExperimentRecord],
    value_fn: impl Fn(&ExperimentRecord) -> (String, f64),
) -> Vec<LabeledMetricPoint> {
    let mut grouped: BTreeMap<String, (f64, usize)> = BTreeMap::new();

    for record in records {
        let (label, value) = value_fn(record);
        let bucket = grouped.entry(label).or_insert((0.0, 0));
        bucket.0 += value;
        bucket.1 += 1;
    }

    grouped
        .into_iter()
        .map(|(label, (total, count))| LabeledMetricPoint {
            label,
            value: total / count as f64,
        })
        .collect()
}

fn draw_dual_line_chart(
    output: &Path,
    title: &str,
    x_label: &str,
    delay_label: &str,
    throughput_label: &str,
    delay_points: &[MetricPoint],
    throughput_points: &[MetricPoint],
) -> Result<()> {
    ensure!(!delay_points.is_empty(), "delay series must not be empty");
    ensure!(
        !throughput_points.is_empty(),
        "throughput series must not be empty"
    );

    let root = BitMapBackend::new(output, CHART_SIZE).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((1, 2));

    draw_line_chart(
        &areas[0],
        &format!("{title}: Delay"),
        x_label,
        delay_label,
        delay_points,
        RED,
    )?;
    draw_line_chart(
        &areas[1],
        &format!("{title}: Throughput"),
        x_label,
        throughput_label,
        throughput_points,
        BLUE,
    )?;

    root.present()?;
    Ok(())
}

fn draw_line_chart(
    area: &DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    title: &str,
    x_label: &str,
    y_label: &str,
    points: &[MetricPoint],
    color: RGBColor,
) -> Result<()> {
    let x_min = points.first().map(|point| point.x).unwrap_or(0.0);
    let x_max = points.last().map(|point| point.x).unwrap_or(1.0);
    let y_max = points
        .iter()
        .map(|point| point.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let x_end = if (x_max - x_min).abs() < f64::EPSILON {
        x_max + 1.0
    } else {
        x_max
    };

    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_end, 0.0_f64..(y_max * 1.1))?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .light_line_style(TRANSPARENT)
        .draw()?;

    chart.draw_series(LineSeries::new(
        points.iter().map(|point| (point.x, point.value)),
        color.stroke_width(3),
    ))?;
    chart.draw_series(
        points
            .iter()
            .map(|point| Circle::new((point.x, point.value), 5, color.filled())),
    )?;

    Ok(())
}

fn draw_dual_bar_chart(
    output: &Path,
    title: &str,
    delay_label: &str,
    throughput_label: &str,
    delay_points: &[LabeledMetricPoint],
    throughput_points: &[LabeledMetricPoint],
) -> Result<()> {
    ensure!(!delay_points.is_empty(), "delay bars must not be empty");
    ensure!(
        !throughput_points.is_empty(),
        "throughput bars must not be empty"
    );

    let root = BitMapBackend::new(output, CHART_SIZE).into_drawing_area();
    root.fill(&WHITE)?;
    let areas = root.split_evenly((1, 2));

    draw_bar_chart(
        &areas[0],
        &format!("{title}: Delay"),
        delay_label,
        delay_points,
        RED,
    )?;
    draw_bar_chart(
        &areas[1],
        &format!("{title}: Throughput"),
        throughput_label,
        throughput_points,
        BLUE,
    )?;

    root.present()?;
    Ok(())
}

fn draw_bar_chart(
    area: &DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    title: &str,
    y_label: &str,
    points: &[LabeledMetricPoint],
    color: RGBColor,
) -> Result<()> {
    let y_max = points
        .iter()
        .map(|point| point.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0..points.len(), 0.0_f64..(y_max * 1.1))?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(points.len())
        .x_label_formatter(&|value| {
            let index = *value;
            points
                .get(index)
                .map(|point| point.label.clone())
                .unwrap_or_default()
        })
        .y_desc(y_label)
        .draw()?;

    chart.draw_series(points.iter().enumerate().map(|(index, point)| {
        Rectangle::new(
            [(index, 0.0), (index + 1, point.value)],
            color.mix(0.7).filled(),
        )
    }))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::app::output::ExperimentRecord;

    use super::{aggregate_labeled_metric, aggregate_numeric_metric};

    fn record(
        total_users: u32,
        cw_min: Option<u32>,
        class_name: &str,
        delay: f64,
        throughput: f64,
    ) -> ExperimentRecord {
        ExperimentRecord {
            scenario: String::from("test"),
            trial: 0,
            seed: 1,
            total_users,
            cw_min,
            lower_cw_min: None,
            higher_cw_min: None,
            class_name: class_name.to_string(),
            class_users: total_users,
            successful_packets: 1,
            collision_attempts: 0,
            average_delay_slots: delay,
            throughput_bits_per_slot: throughput,
        }
    }

    #[test]
    fn aggregates_numeric_metrics_by_x_value() {
        let records = vec![
            record(10, Some(8), "standard", 10.0, 100.0),
            record(10, Some(8), "standard", 14.0, 200.0),
            record(20, Some(16), "standard", 22.0, 300.0),
        ];

        let points = aggregate_numeric_metric(&records, |record| {
            Some((record.total_users as f64, record.average_delay_slots))
        });

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].x, 10.0);
        assert_eq!(points[0].value, 12.0);
        assert_eq!(points[1].x, 20.0);
        assert_eq!(points[1].value, 22.0);
    }

    #[test]
    fn aggregates_labeled_metrics_by_class_name() {
        let records = vec![
            record(20, None, "lower-cw", 50.0, 1000.0),
            record(20, None, "higher-cw", 150.0, 300.0),
            record(20, None, "lower-cw", 70.0, 1100.0),
        ];

        let points = aggregate_labeled_metric(&records, |record| {
            (record.class_name.clone(), record.average_delay_slots)
        });

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].label, "higher-cw");
        assert_eq!(points[0].value, 150.0);
        assert_eq!(points[1].label, "lower-cw");
        assert_eq!(points[1].value, 60.0);
    }
}
