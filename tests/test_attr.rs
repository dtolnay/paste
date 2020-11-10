use paste::paste;

#[test]
fn test_paste_cfg() {
    macro_rules! m {
        ($ret:ident, $width:expr) => {
            paste! {
                #[cfg(any(feature = "protocol_feature_" $ret:snake, target_pointer_width = "" $width))]
                fn new() -> $ret { todo!() }
            }
        };
    }

    struct Paste;

    #[cfg(target_pointer_width = "64")]
    m!(Paste, 64);
    #[cfg(target_pointer_width = "32")]
    m!(Paste, 32);

    let _ = new;
}
