use std::path::Path;

use crate::plot::{PlotData, PlotParameter};

pub fn load_csv(path: &Path) -> Result<PlotData, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read CSV: {}", e))?;

    let mut lines = content.lines();
    let header = lines.next().ok_or("CSV file is empty")?;
    let columns: Vec<&str> = header.split(',').map(|s| s.trim()).collect();

    let mut data: Vec<Vec<f64>> = vec![vec![]; columns.len()];

    for (line_num, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').collect();
        if values.len() != columns.len() {
            return Err(format!(
                "Row {} has {} columns, expected {}",
                line_num + 2,
                values.len(),
                columns.len()
            ));
        }
        for (i, val) in values.iter().enumerate() {
            let v: f64 = val.trim().parse().map_err(|_| {
                format!(
                    "Invalid number '{}' at row {}, col {}",
                    val,
                    line_num + 2,
                    i + 1
                )
            })?;
            data[i].push(v);
        }
    }

    let mut plot_data = PlotData::new();
    for (i, col) in columns.iter().enumerate() {
        plot_data.insert(col.to_string(), PlotParameter::FloatArray(data[i].clone()));
    }

    Ok(plot_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_csv() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_scatter.csv");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "x,y").unwrap();
        writeln!(f, "1.0,2.0").unwrap();
        writeln!(f, "3.0,4.0").unwrap();

        let data = load_csv(&path).expect("Should load CSV");
        assert!(data.contains("x"));
        assert!(data.contains("y"));
    }
}
