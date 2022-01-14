#[cfg(test)]
mod tests {
    use actix_easy_multipart::FromMultipart;
    use actix_easy_multipart::{MultipartField, MultipartText, Multiparts};
    use std::convert::TryFrom;

    #[derive(FromMultipart, Debug)]
    struct Test {
        string: String,
        none_string: Option<String>,
        some_string: Option<String>,
        int: i32,
        float: f64,
        int_array: Vec<i32>,
        // file: MultipartFile,
        // optional_file: Option<MultipartFile>,
        // file_array: MultipartFile,
    }

    #[test]
    fn it_works() {
        let mut m = Multiparts::new();
        m.push(MultipartField::Text(MultipartText {
            name: "string".to_string(),
            text: "Hello World".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "some_string".to_string(),
            text: "Hello World".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "int".to_string(),
            text: "69".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "float".to_string(),
            text: "-1.25".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "int_array".to_string(),
            text: "2".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "int_array".to_string(),
            text: "4".to_string(),
        }));
        m.push(MultipartField::Text(MultipartText {
            name: "int_array".to_string(),
            text: "6".to_string(),
        }));
        let result = match Test::try_from(m) {
            Ok(r) => r,
            Err(e) => panic!("{}", e),
        };

        assert_eq!(result.string, "Hello World".to_string());
        assert_eq!(result.none_string, None);
        assert_eq!(result.some_string, Some("Hello World".to_string()));
        assert_eq!(result.int, 69);
        assert_eq!(result.float, -1.25);
        assert_eq!(result.int_array, vec![2, 4, 6]);
    }
}
