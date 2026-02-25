pub mod async_redlock;
pub mod redlock;

pub const DEL: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
	return redis.call("DEL", KEYS[1])
else
	return 0
end
"#;
