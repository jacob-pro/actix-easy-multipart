#[cfg(test)]
mod tests {
    use actix_easy_multipart::deserialize::Error;
    use actix_easy_multipart::load::GroupedFields;
    use actix_easy_multipart::{Field, Text};
    use actix_easy_multipart::{File, FromMultipart};
    use std::convert::TryFrom;
    use tempfile::NamedTempFile;

    #[derive(FromMultipart)]
    struct TestVariety {
        string: String,
        none_string: Option<String>,
        some_string: Option<String>,
        int: i32,
        float: f64,
        int_array: Vec<i32>,
        file: File,
        none_file: Option<File>,
        some_file: Option<File>,
        file_array: Vec<File>,
    }

    fn mock_text(name: &str, text: &str) -> Field {
        Field::Text(Text {
            name: name.to_string(),
            text: text.to_string(),
        })
    }

    fn mock_file(name: &str, size: usize) -> Field {
        Field::File(File {
            file: NamedTempFile::new().unwrap(),
            size,
            name: name.to_string(),
            filename: None,
            mime: mime::APPLICATION_OCTET_STREAM,
        })
    }

    fn mock_form(parts: Vec<Field>) -> GroupedFields {
        parts
            .into_iter()
            .map(|field| (field.name().to_owned(), field))
            .collect()
    }

    #[test]
    fn test_variety() {
        let form = mock_form(vec![
            mock_text("string", "Hello World"),
            mock_text("some_string", "Hello World"),
            mock_text("int", "69"),
            mock_text("float", "-1.25"),
            mock_text("int_array", "2"),
            mock_text("int_array", "4"),
            mock_text("int_array", "6"),
            mock_file("file", 10),
            mock_file("some_file", 20),
            mock_file("file_array", 30),
            mock_file("file_array", 30),
        ]);
        let result = TestVariety::try_from(form).unwrap();
        assert_eq!(result.string, "Hello World".to_string());
        assert_eq!(result.none_string, None);
        assert_eq!(result.some_string, Some("Hello World".to_string()));
        assert_eq!(result.int, 69);
        assert_eq!(result.float, -1.25);
        assert_eq!(result.int_array, vec![2, 4, 6]);
        assert_eq!(result.file.size, 10);
        assert!(result.none_file.is_none());
        assert!(result.some_file.is_some());
        assert_eq!(result.file_array.len(), 2)
    }

    #[derive(FromMultipart, Debug)]
    struct AllowsExtras {
        string: String,
        option_string: Option<String>,
        file: File,
        option_file: Option<File>,
    }

    #[test]
    fn test_allows_extras() {
        let form = mock_form(vec![
            mock_text("string", "One"),
            mock_text("string", "Two"),
            mock_file("string", 10),
            mock_text("option_string", "One"),
            mock_text("option_string", "Two"),
            mock_file("file", 10),
            mock_file("file", 20),
            mock_text("file", "file"),
            mock_file("option_file", 10),
            mock_file("option_file", 20),
            mock_text("erroneous", "erroneous"),
        ]);
        let result = AllowsExtras::try_from(form).unwrap();
        assert_eq!(result.string, "Two".to_string());
        assert_eq!(result.option_string, Some("Two".to_string()));
        assert_eq!(result.file.size, 20);
        assert_eq!(result.option_file.unwrap().size, 20);
    }

    #[derive(FromMultipart)]
    struct TestMissing {
        _string: String,
        _file: File,
    }

    #[test]
    fn test_missing() {
        let missing_file = mock_form(vec![mock_text("_string", "One")]);
        match TestMissing::try_from(missing_file) {
            Err(Error::FileNotFound(f)) if f == "_file" => {}
            _ => panic!("Unexpected result"),
        }

        let missing_string = mock_form(vec![mock_file("_file", 10)]);
        match TestMissing::try_from(missing_string) {
            Err(Error::TextNotFound(f)) if f == "_string" => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[derive(FromMultipart)]
    #[from_multipart(deny_extra_parts)]
    struct DeniesExtras1 {
        _string: String,
    }
}
