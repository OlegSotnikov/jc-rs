//! Line-based slicing with Python-style semantics, ported from jc's `line_slice`.

/// Slice a vector of lines using Python-style start/end indices.
///
/// - `start`: inclusive start index (None = from beginning)
/// - `end`: exclusive end index (None = to end)
/// - Positive indices are handled without full materialization.
/// - Negative indices require full materialization (Python semantics).
pub fn slice_lines(lines: Vec<String>, start: Option<i64>, end: Option<i64>) -> Vec<String> {
    if start.is_none() && end.is_none() {
        return lines;
    }

    let s = start.unwrap_or(0);
    let e = end;

    // Positive-only slice: can work without collecting everything
    let all_positive = s >= 0 && e.is_none_or(|v| v >= 0);

    if all_positive {
        let s = s as usize;
        let iter = lines.into_iter().skip(s);
        if let Some(end_val) = e {
            let end_val = end_val as usize;
            if end_val <= s {
                return Vec::new();
            }
            iter.take(end_val - s).collect()
        } else {
            iter.collect()
        }
    } else {
        // Negative indices — materialize all lines
        let len = lines.len() as i64;

        let start_idx = if s < 0 {
            (len + s).max(0) as usize
        } else {
            s as usize
        };

        let end_idx = match e {
            None => len as usize,
            Some(v) if v < 0 => (len + v).max(0) as usize,
            Some(v) => v as usize,
        };

        if end_idx <= start_idx {
            return Vec::new();
        }

        lines
            .into_iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_slice_none() {
        let l = lines(&["a", "b", "c"]);
        assert_eq!(slice_lines(l, None, None), lines(&["a", "b", "c"]));
    }

    #[test]
    fn test_slice_positive_start() {
        let l = lines(&["a", "b", "c", "d"]);
        assert_eq!(slice_lines(l, Some(1), None), lines(&["b", "c", "d"]));
    }

    #[test]
    fn test_slice_positive_end() {
        let l = lines(&["a", "b", "c", "d"]);
        assert_eq!(slice_lines(l, None, Some(2)), lines(&["a", "b"]));
    }

    #[test]
    fn test_slice_positive_both() {
        let l = lines(&["a", "b", "c", "d"]);
        assert_eq!(slice_lines(l, Some(1), Some(3)), lines(&["b", "c"]));
    }

    #[test]
    fn test_slice_negative_end() {
        let l = lines(&["a", "b", "c", "d"]);
        assert_eq!(slice_lines(l, None, Some(-1)), lines(&["a", "b", "c"]));
    }

    #[test]
    fn test_slice_negative_start() {
        let l = lines(&["a", "b", "c", "d"]);
        assert_eq!(slice_lines(l, Some(-2), None), lines(&["c", "d"]));
    }

    #[test]
    fn test_slice_empty_range() {
        let l = lines(&["a", "b", "c"]);
        assert_eq!(slice_lines(l, Some(2), Some(1)), lines(&[]));
    }

    #[test]
    fn test_slice_zero_start() {
        let l = lines(&["a", "b", "c"]);
        assert_eq!(slice_lines(l, Some(0), Some(2)), lines(&["a", "b"]));
    }
}
