
/// Extract and parse bencode dictionary values with automatic error handling.
///
/// This macro eliminates boilerplate when extracting typed values from bencoded dictionaries.
/// It handles type checking and provides consistent error messages for missing/wrong-type fields.
///
/// Four forms:
/// - `required $key, Type => $parse_fn`: Extract required field and parse with non-fallible function
/// - `optional $key, Type => $parse_fn`: Extract optional field, returns `Option<T>` with non-fallible parse
/// - `required $key, Type => errors $parse_fn`: Extract required field with fallible parser returning `Result<T, StructureError>`
/// - `optional $key, Type => errors $parse_fn`: Extract optional field with fallible parser, returns `Option<T>`
///
/// # Example
/// ```ignore
/// // Non-fallible parsing (simple value transformation)
/// let name = bencode_get!(dict: required "name", ByteString => |x| x.to_string())?;
///
/// // Fallible parsing (with validation)
/// let length = bencode_get!(dict: required "length", Int => errors |len: &i64| {
///     if *len < 0 { return Err(StructureError::NegativeLength) }
///     Ok(*len as u64)
/// })?;
/// ```
#[macro_export]
macro_rules! bencode_get {
    ($dict:tt : required $key:expr, $element_type:ident => $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => Ok($parse_fn(val)),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Err(StructureError::RequiredKeyMissing($key.to_string())),
        }
    }};
    ($dict:tt : optional $key:expr, $element_type:ident => $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => Ok(Some($parse_fn(val))),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Ok(None),
        }
    }};
    ($dict:tt : required $key:expr, $element_type:ident => errors $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => $parse_fn(val),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Err(StructureError::RequiredKeyMissing($key.to_string())),
        }
    }};
    ($dict:tt : optional $key:expr, $element_type:ident => errors $parse_fn:expr) => {{
        use BencodeElement::*;

        match $dict.get(&$key.into()) {
            Some($element_type(val)) => $parse_fn(val).map(Some),
            Some(_) => Err(StructureError::WrongType($key.to_string())),
            None => Ok(None),
        }
    }};
}
