#[macro_export]
macro_rules! ok_or {
    ($result: expr, $err: pat => $block: expr) => {
        match $result {
            Ok(val) => val,
            Err($err) => $block,
        }
    };
}

#[macro_export]
macro_rules! some_or {
    ($option: expr, $block: expr) => {
        match $option {
            Some(val) => val,
            None => $block,
        }
    };
}

#[macro_export]
macro_rules! none_or {
    ($option: expr, $val: pat => $block: expr) => {
        match $option {
            None => None,
            Some($val) => $block,
        }
    };
}
