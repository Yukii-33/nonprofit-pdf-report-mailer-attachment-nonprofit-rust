use std::fmt::Write;

/// Builds a small, valid one-page PDF without invoking a system renderer.
pub fn one_page(title: &str, lines: &[String]) -> Vec<u8> {
    let mut content = String::from("BT /F1 18 Tf 54 740 Td ");
    write!(&mut content, "({}) Tj", pdf_text(title)).unwrap();
    for line in lines {
        write!(&mut content, " 0 -24 Td /F1 11 Tf ({}) Tj", pdf_text(line)).unwrap();
    }
    content.push_str(" ET");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn pdf_text(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '(' => "\\(".to_owned(),
            ')' => "\\)".to_owned(),
            '\\' => "\\\\".to_owned(),
            c if c.is_ascii() && !c.is_control() => c.to_string(),
            _ => "?".to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_pdf_header_and_cross_reference() {
        let bytes = one_page("Receipt", &["Donation: $25.00".to_owned()]);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.windows(4).any(|part| part == b"xref"));
    }
}
