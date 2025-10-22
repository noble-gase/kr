use anyhow::Ok;
use bon::builder;
use time::macros::offset;

pub const DATE: &str = "[year]-[month]-[day]";
pub const TIME: &str = "[hour]:[minute]:[second]";
pub const DATE_TIME: &str = "[year]-[month]-[day] [hour]:[minute]:[second]";

/// 获取当前时间
pub fn now(offset: Option<time::UtcOffset>) -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc().to_offset(offset.unwrap_or(offset!(+8)))
}

/// 根据时间字符串生成时间对象
///
/// # Example
///
/// ```
/// let time = times::parse().datetime("2023-07-12 00:00:00").format(times::DATE_TIME).call().unwrap();
/// ```
#[builder]
pub fn parse(
    datetime: impl AsRef<str>,
    format: Option<&str>,
    offset: Option<time::UtcOffset>,
) -> anyhow::Result<time::OffsetDateTime> {
    let desc = time::format_description::parse(format.unwrap_or(DATE_TIME))?;
    let v = time::PrimitiveDateTime::parse(datetime.as_ref(), &desc)?
        .assume_offset(offset.unwrap_or(offset!(+8)));
    Ok(v)
}

/// 根据Unix时间戳生成时间对象
///
/// # Example
///
/// ```
/// let time = times::from_timestamp().timestamp(1689140713).call().unwrap();
/// ```
#[builder]
pub fn from_timestamp(
    timestamp: i64,
    offset: Option<time::UtcOffset>,
) -> anyhow::Result<time::OffsetDateTime> {
    let off = offset.unwrap_or(offset!(+8));
    if timestamp < 0 {
        return Ok(time::OffsetDateTime::now_utc().to_offset(off));
    }
    let v = time::OffsetDateTime::from_unix_timestamp(timestamp)?.to_offset(off);
    Ok(v)
}

/// Unix时间戳格式化
///
/// # Example
///
/// ```
/// let time = times::to_string().timestamp(1689140713).format(times::DATE_TIME).call().unwrap();
/// ```
#[builder]
pub fn to_string(
    timestamp: i64,
    format: Option<&str>,
    offset: Option<time::UtcOffset>,
) -> anyhow::Result<String> {
    let desc = time::format_description::parse(format.unwrap_or(DATE_TIME))?;
    let off = offset.unwrap_or(offset!(+8));
    if timestamp < 0 {
        let v = time::OffsetDateTime::now_utc()
            .to_offset(off)
            .format(&desc)?;
        return Ok(v);
    }
    let v = time::OffsetDateTime::from_unix_timestamp(timestamp)?
        .to_offset(off)
        .format(&desc)?;
    Ok(v)
}

/// 日期转Unix时间戳
///
/// # Example
///
/// ```
/// let time = times::to_timestamp().datetime("2023-07-12 13:45:13").format(times::DATE_TIME).call().unwrap();
/// ```
#[builder]
pub fn to_timestamp(
    datetime: impl AsRef<str>,
    format: Option<&str>,
    offset: Option<time::UtcOffset>,
) -> anyhow::Result<i64> {
    if datetime.as_ref().is_empty() {
        return Ok(0);
    }
    let desc = time::format_description::parse(format.unwrap_or(DATE_TIME))?;
    let v = time::PrimitiveDateTime::parse(datetime.as_ref(), &desc)?
        .assume_offset(offset.unwrap_or(offset!(+8)))
        .unix_timestamp();
    Ok(v)
}

#[cfg(test)]
mod tests {
    use crate::times;

    #[test]
    fn parse() {
        // date
        assert_eq!(
            times::parse()
                .datetime("2023-07-12 00:00:00")
                .format(times::DATE_TIME)
                .call()
                .unwrap()
                .unix_timestamp(),
            1689091200
        );
        assert_eq!(
            times::parse()
                .datetime("2023/07/12 00:00:00")
                .format("[year]/[month]/[day] [hour]:[minute]:[second]")
                .call()
                .unwrap()
                .unix_timestamp(),
            1689091200
        );

        // datetime
        assert_eq!(
            times::parse()
                .datetime("2023-07-12 13:45:13")
                .format(times::DATE_TIME)
                .call()
                .unwrap()
                .unix_timestamp(),
            1689140713
        );
        assert_eq!(
            times::parse()
                .datetime("2023/07/12 13:45:13")
                .format("[year]/[month]/[day] [hour]:[minute]:[second]")
                .call()
                .unwrap()
                .unix_timestamp(),
            1689140713
        );
    }

    #[test]
    fn from_timestamp() {
        assert_eq!(
            times::from_timestamp()
                .timestamp(1689140713)
                .call()
                .unwrap()
                .unix_timestamp(),
            1689140713
        )
    }

    #[test]
    fn time_to_str() {
        // date
        assert_eq!(
            times::to_string()
                .format(times::DATE)
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "2023-07-12"
        );
        assert_eq!(
            times::to_string()
                .format("[year]/[month]/[day]")
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "2023/07/12"
        );

        // time
        assert_eq!(
            times::to_string()
                .format(times::TIME)
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "13:45:13"
        );
        assert_eq!(
            times::to_string()
                .format("[hour]-[minute]-[second]")
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "13-45-13"
        );

        // datetime
        assert_eq!(
            times::to_string()
                .format(times::DATE_TIME)
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "2023-07-12 13:45:13"
        );
        assert_eq!(
            times::to_string()
                .format("[year]/[month]/[day] [hour]:[minute]:[second]")
                .timestamp(1689140713)
                .call()
                .unwrap(),
            "2023/07/12 13:45:13"
        );
    }

    #[test]
    fn str_to_time() {
        // date
        assert_eq!(
            times::to_timestamp()
                .format(times::DATE_TIME)
                .datetime("2023-07-12 00:00:00")
                .call()
                .unwrap(),
            1689091200
        );
        assert_eq!(
            times::to_timestamp()
                .format("[year]/[month]/[day] [hour]:[minute]:[second]")
                .datetime("2023/07/12 00:00:00")
                .call()
                .unwrap(),
            1689091200
        );

        // datetime
        assert_eq!(
            times::to_timestamp()
                .format("[year]-[month]-[day] [hour]:[minute]")
                .datetime("2023-07-12 13:45")
                .call()
                .unwrap(),
            1689140700
        );
        assert_eq!(
            times::to_timestamp()
                .format(times::DATE_TIME)
                .datetime("2023-07-12 13:45:13")
                .call()
                .unwrap(),
            1689140713
        );
        assert_eq!(
            times::to_timestamp()
                .format("[year]/[month]/[day] [hour]:[minute]:[second]")
                .datetime("2023/07/12 13:45:13")
                .call()
                .unwrap(),
            1689140713
        );
    }
}
