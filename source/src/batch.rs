//! Expand direct URLs and inclusive numeric/alphabetic URL ranges.
//!
//! One URL template per line; multiple range groups form a Cartesian product.
//! Numeric ranges preserve explicit zero padding. Literal brackets without a
//! dash are left untouched, including IPv6 addresses. URL/protocol validation
//! remains the caller's responsibility, after expansion.

const MAX_URLS: usize = 1_000;

/// Expand `file[001-025].zip`, `[a-z]`, or `[A-Z]` in URL templates.
///
/// Empty lines are ignored. The total result across all lines is bounded at
/// 1,000 URLs. Invalid, reversed, mixed-case, and oversized ranges are errors;
/// callers never receive a silently truncated or partly expanded list.
pub fn expand_urls(input: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.chars().any(char::is_whitespace) {
            return Err(format!(
                "Line {}: enter one URL per line; encode spaces as %20.",
                line_index + 1
            ));
        }
        let remaining = MAX_URLS - result.len();
        if remaining == 0 {
            return Err(format!("A batch can contain at most {MAX_URLS} URLs."));
        }
        let expanded = expand_line(line, remaining)
            .map_err(|error| format!("Line {}: {error}", line_index + 1))?;
        result.extend(expanded);
    }
    if result.is_empty() {
        return Err("Enter at least one download URL.".into());
    }
    Ok(result)
}

fn expand_line(line: &str, limit: usize) -> Result<Vec<String>, String> {
    let mut expanded = vec![String::new()];
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find('[') {
        let start = cursor + relative;
        let Some(close) = line[start + 1..].find(']').map(|n| start + 1 + n) else {
            return Err("Unclosed '[' in URL template.".into());
        };
        let range = &line[start + 1..close];
        if range.contains('[') {
            return Err("Nested brackets are not supported in URL ranges.".into());
        }
        let prefix = &line[cursor..start];
        if !range.contains('-') {
            // Normal URL syntax such as http://[::1]/ is not a range.
            for item in &mut expanded {
                item.push_str(prefix);
                item.push_str(&line[start..=close]);
            }
            cursor = close + 1;
            continue;
        }
        let values = range_values(range, limit)?;
        let count = expanded
            .len()
            .checked_mul(values.len())
            .filter(|&n| n <= limit)
            .ok_or_else(|| format!("Expansion exceeds the {MAX_URLS}-URL batch limit."))?;
        let mut next = Vec::with_capacity(count);
        for item in &expanded {
            for value in &values {
                let mut item = item.clone();
                item.push_str(prefix);
                item.push_str(value);
                next.push(item);
            }
        }
        expanded = next;
        cursor = close + 1;
    }
    for item in &mut expanded {
        item.push_str(&line[cursor..]);
    }
    Ok(expanded)
}

fn range_values(range: &str, limit: usize) -> Result<Vec<String>, String> {
    let (first, last) = range
        .split_once('-')
        .ok_or_else(|| "Use [1-10], [001-010], [a-z], or [A-Z].".to_string())?;
    if first.is_empty() || last.is_empty() || last.contains('-') {
        return Err(format!("Invalid range [{range}]."));
    }
    if first.bytes().all(|c| c.is_ascii_digit()) && last.bytes().all(|c| c.is_ascii_digit()) {
        let start = first
            .parse::<u64>()
            .map_err(|_| "Range start is too large.")?;
        let end = last.parse::<u64>().map_err(|_| "Range end is too large.")?;
        let count = end
            .checked_sub(start)
            .and_then(|difference| difference.checked_add(1))
            .ok_or_else(|| {
                format!("Range [{range}] must ascend and fit within the batch limit.")
            })?;
        if count > limit as u64 {
            return Err(format!("Expansion exceeds the {MAX_URLS}-URL batch limit."));
        }
        let padded = (first.len() > 1 && first.starts_with('0'))
            || (last.len() > 1 && last.starts_with('0'));
        let width = if padded {
            first.len().max(last.len())
        } else {
            0
        };
        return Ok((start..=end)
            .map(|number| format!("{number:0width$}"))
            .collect());
    }
    if first.len() == 1 && last.len() == 1 {
        let start = first.as_bytes()[0];
        let end = last.as_bytes()[0];
        let same_case = (start.is_ascii_lowercase() && end.is_ascii_lowercase())
            || (start.is_ascii_uppercase() && end.is_ascii_uppercase());
        if same_case && start <= end {
            if usize::from(end - start) + 1 > limit {
                return Err(format!("Expansion exceeds the {MAX_URLS}-URL batch limit."));
            }
            return Ok((start..=end)
                .map(|letter| char::from(letter).to_string())
                .collect());
        }
    }
    Err(format!(
        "Invalid range [{range}]. Use ascending numbers or letters of the same case."
    ))
}

#[cfg(test)]
mod tests {
    use super::expand_urls;

    #[test]
    fn direct_urls_blank_lines_and_ipv6_remain_intact() {
        assert_eq!(
            expand_urls("\n https://example.test/a%20b.zip \nhttp://[::1]/x\n").unwrap(),
            ["https://example.test/a%20b.zip", "http://[::1]/x"]
        );
        assert!(expand_urls(" \n ").is_err());
        assert!(expand_urls("https://example.test/a b.zip").is_err());
    }

    #[test]
    fn numbers_preserve_explicit_padding_but_do_not_invent_it() {
        assert_eq!(
            expand_urls("https://host/file[009-011].zip").unwrap(),
            [
                "https://host/file009.zip",
                "https://host/file010.zip",
                "https://host/file011.zip"
            ]
        );
        assert_eq!(
            expand_urls("https://host/[9-10]").unwrap(),
            ["https://host/9", "https://host/10"]
        );
    }

    #[test]
    fn multiple_groups_and_lines_expand_in_stable_order() {
        assert_eq!(
            expand_urls("https://host/[a-b]/[01-02]\nhttps://host/[X-Y]\nhttps://host/direct")
                .unwrap(),
            [
                "https://host/a/01",
                "https://host/a/02",
                "https://host/b/01",
                "https://host/b/02",
                "https://host/X",
                "https://host/Y",
                "https://host/direct"
            ]
        );
    }

    #[test]
    fn malformed_descending_and_overflow_ranges_fail() {
        for template in [
            "[2-1]",
            "[z-a]",
            "[a-Z]",
            "[a-9]",
            "[1-]",
            "[-1]",
            "[1-2-3]",
            "[[1-2]]",
            "[1-2",
            "[0-18446744073709551615]",
            "[1-18446744073709551616]",
        ] {
            assert!(
                expand_urls(&format!("https://host/{template}")).is_err(),
                "{template}"
            );
        }
    }

    #[test]
    fn total_limit_covers_cartesian_products_and_direct_urls() {
        assert_eq!(expand_urls("https://host/[1-1000]").unwrap().len(), 1000);
        assert!(expand_urls("https://host/[1-1001]").is_err());
        assert!(expand_urls("https://host/[1-40]/[1-30]").is_err());
        assert!(expand_urls("https://host/[1-1000]\nhttps://host/extra").is_err());
        assert_eq!(
            expand_urls("https://host/[1-25]/[1-40]").unwrap().len(),
            1000
        );
    }
}
