//! `lingara-embed-uniffi-bindgen generate --library <cdylib> --language kotlin --no-format`
//! (ADR 1.10.26ae D2): `make codegen-kotlin` and `check-codegen-kotlin` run it.

fn main() {
    uniffi::uniffi_bindgen_main()
}
