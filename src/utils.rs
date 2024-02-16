use eyre::{eyre, Result};

pub fn sdk_dec_string_to_f64(s: String) -> Result<f64> {
    let big_int = s
        .parse::<f64>()
        .map_err(|e| eyre!("error parsing f64 from string: {e:?}"))?;

    Ok(big_int as f64 / 10_f64.powi(18))
}
