use std::fs::File;
use std::io::{BufReader, BufWriter, Write, BufRead};
use std::path::Path;
use crate::body::Body;

/// Read simulation state from a file
pub fn read_bodies<P: AsRef<Path>>(
    path: P
) -> Result<Vec<Body>, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read header information
    let timestep: f64 = lines.next()
        .ok_or("Missing timestep")?
        .map_err(|e| format!("Failed to read timestep: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Invalid timestep format: {}", e))?;

    let g: f64 = lines.next()
        .ok_or("Missing G value")?
        .map_err(|e| format!("Failed to read G value: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Invalid G value format: {}", e))?;

    let softening: f64 = lines.next()
        .ok_or("Missing softening factor")?
        .map_err(|e| format!("Failed to read softening factor: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Invalid softening factor format: {}", e))?;

    let tree_ratio: f64 = lines.next()
        .ok_or("Missing tree ratio")?
        .map_err(|e| format!("Failed to read tree ratio: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Invalid tree ratio format: {}", e))?;

    let n_bodies: usize = lines.next()
        .ok_or("Missing number of bodies")?
        .map_err(|e| format!("Failed to read number of bodies: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Invalid number of bodies format: {}", e))?;

    // Read body data
    let mut bodies = Vec::with_capacity(n_bodies);
    for (i, line) in lines.enumerate() {
        if i >= n_bodies {
            break;
        }

        let line = line.map_err(|e| format!("Failed to read body data: {}", e))?;
        let parts: Vec<f64> = line.split_whitespace()
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<f64>, _>>()
            .map_err(|e| format!("Invalid body data format: {}", e))?;

        if parts.len() != 5 {
            return Err(format!("Invalid body data: expected 5 values, got {}", parts.len()));
        }

        bodies.push(Body::new(
            parts[0], // mass
            parts[1], // x
            parts[2], // y
            parts[3], // vx
            parts[4], // vy
        ));
    }

    if bodies.len() != n_bodies {
        return Err(format!(
            "Mismatch in body count: expected {}, got {}",
            n_bodies,
            bodies.len()
        ));
    }

    Ok(bodies)
}

/// Write simulation state to a file
pub fn write_bodies<P: AsRef<Path>>(
    path: P,
    bodies: &[Body],
    timestep: f64,
    g: f64,
    softening: f64,
    tree_ratio: f64,
) -> Result<(), String> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory structure: {}", e))?;
    }

    // Open file with proper error handling
    let file = File::create(path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    let mut writer = BufWriter::new(file);

    // Write header information
    writeln!(writer, "{:.16e}", timestep)
        .map_err(|e| format!("Failed to write timestep: {}", e))?;
    writeln!(writer, "{:.16e}", g)
        .map_err(|e| format!("Failed to write G value: {}", e))?;
    writeln!(writer, "{:.16e}", softening)
        .map_err(|e| format!("Failed to write softening factor: {}", e))?;
    writeln!(writer, "{:.16e}", tree_ratio)
        .map_err(|e| format!("Failed to write tree ratio: {}", e))?;
    writeln!(writer, "{}", bodies.len())
        .map_err(|e| format!("Failed to write body count: {}", e))?;

    // Write body data
    for body in bodies {
        writeln!(
            writer,
            "{:.16e} {:.16e} {:.16e} {:.16e} {:.16e}",
            body.mass,
            body.position[0],
            body.position[1],
            body.velocity[0],
            body.velocity[1]
        ).map_err(|e| format!("Failed to write body data: {}", e))?;
    }

    // Ensure all data is written
    writer.flush()
        .map_err(|e| format!("Failed to flush file buffer: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_write_and_read_bodies() -> Result<(), String> {
        // Create a temporary directory for the test
        let dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let file_path = dir.path().join("test_bodies.dat");

        // Create test bodies
        let original_bodies = vec![
            Body::new(1.0, 0.0, 0.0, 0.0, 0.0),
            Body::new(2.0, 1.0, 1.0, -0.1, 0.1),
        ];

        // Test parameters
        let timestep = 0.1;
        let g = 1.0;
        let softening = 0.001;
        let tree_ratio = 0.5;

        // Write bodies to file
        write_bodies(
            &file_path,
            &original_bodies,
            timestep,
            g,
            softening,
            tree_ratio,
        )?;

        // Read bodies back
        let read_bodies = read_bodies(&file_path)?;

        // Verify data
        assert_eq!(read_bodies.len(), original_bodies.len());
        for (original, read) in original_bodies.iter().zip(read_bodies.iter()) {
            assert_eq!(original.mass, read.mass);
            assert_eq!(original.position, read.position);
            assert_eq!(original.velocity, read.velocity);
        }

        // Clean up
        dir.close().map_err(|e| format!("Failed to clean up temp dir: {}", e))?;
        
        Ok(())
    }

    #[test]
    fn test_invalid_file() {
        let result = read_bodies("nonexistent_file.dat");
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_file() -> Result<(), String> {
        let dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let file_path = dir.path().join("malformed.dat");

        // Create malformed file
        fs::write(&file_path, "not a valid file format")
            .map_err(|e| format!("Failed to write test file: {}", e))?;

        let result = read_bodies(&file_path);
        assert!(result.is_err());

        dir.close().map_err(|e| format!("Failed to clean up temp dir: {}", e))?;
        
        Ok(())
    }
}