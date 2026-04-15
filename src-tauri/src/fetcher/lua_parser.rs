use mlua::prelude::*;
use serde_json::{Map, Value};

pub fn eval_lua_module(source: &str) -> Result<Value, String> {
    let lua = Lua::new();

    // Provide a smart require() that returns stub modules with common utility functions.
    // Wiki modules use require('Module:Table'), require('Module:Math'), etc.
    lua.load(r#"
        local _stubs = {
            ["Module:Table"] = {
                size = function(t)
                    local count = 0
                    for _ in pairs(t) do count = count + 1 end
                    return count
                end,
                indexOf = function(t, val)
                    for i, v in ipairs(t) do if v == val then return i end end
                    return nil
                end,
                contains = function(t, val)
                    for _, v in pairs(t) do if v == val then return true end end
                    return false
                end,
                keys = function(t)
                    local result = {}
                    for k in pairs(t) do result[#result+1] = k end
                    return result
                end,
            },
            ["Module:Math"] = {
                round = function(x, n)
                    n = n or 0
                    local mult = 10^n
                    return math.floor(x * mult + 0.5) / mult
                end,
            },
        }
        function require(name)
            return _stubs[name] or {}
        end
    "#).exec().map_err(|e| format!("failed to set up require stubs: {e}"))?;

    // Override string.format to handle float-to-integer coercion (Lua 5.4 is strict,
    // but wiki modules pass floats to %X/%d formats expecting automatic truncation)
    lua.load(r#"
        local orig_format = string.format
        string.format = function(fmt, ...)
            local args = {...}
            local new_args = {}
            local i = 0
            fmt:gsub("%%[%-%+ #0]*%d*%.?%d*([diouxXeEfgGqscpaA%%])", function(spec)
                i = i + 1
                if (spec == "d" or spec == "i" or spec == "o" or spec == "u" or spec == "x" or spec == "X") and type(args[i]) == "number" then
                    new_args[i] = math.floor(args[i])
                else
                    new_args[i] = args[i]
                end
            end)
            for j = 1, #args do
                if new_args[j] == nil then new_args[j] = args[j] end
            end
            return orig_format(fmt, table.unpack(new_args))
        end
    "#).exec().map_err(|e| format!("failed to patch string.format: {e}"))?;

    let table: LuaTable = lua
        .load(source)
        .eval()
        .map_err(|e| format!("Lua eval error: {e}"))?;

    lua_table_to_json(&table)
}

fn lua_table_to_json(table: &LuaTable) -> Result<Value, String> {
    let len = table.raw_len();

    // Check if array-like (sequential integer keys starting at 1)
    if len > 0 {
        let mut arr = Vec::with_capacity(len as usize);
        let mut is_array = true;
        for i in 1..=len {
            match table.raw_get::<LuaValue>(i) {
                Ok(val) if val != LuaValue::Nil => {
                    arr.push(lua_value_to_json(val)?);
                }
                _ => {
                    is_array = false;
                    break;
                }
            }
        }
        if is_array && arr.len() == len as usize {
            return Ok(Value::Array(arr));
        }
    }

    // Object
    let mut map = Map::new();
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let (key, value) = pair.map_err(|e| format!("table iteration error: {e}"))?;
        let key_str = match &key {
            LuaValue::String(s) => s.to_str().map_err(|e| format!("key encode error: {e}"))?.to_string(),
            LuaValue::Integer(i) => i.to_string(),
            _ => continue,
        };
        map.insert(key_str, lua_value_to_json(value)?);
    }
    Ok(Value::Object(map))
}

fn lua_value_to_json(value: LuaValue) -> Result<Value, String> {
    match value {
        LuaValue::Nil => Ok(Value::Null),
        LuaValue::Boolean(b) => Ok(Value::Bool(b)),
        LuaValue::Integer(i) => Ok(Value::Number(i.into())),
        LuaValue::Number(f) => Ok(serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)),
        LuaValue::String(s) => Ok(Value::String(
            s.to_str().map_err(|e| format!("string encode error: {e}"))?.to_string(),
        )),
        LuaValue::Table(t) => lua_table_to_json(&t),
        _ => Ok(Value::Null), // Functions, userdata, etc. → null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table() {
        let data = eval_lua_module(r#"return { Name = "Excalibur", Health = 100 }"#).unwrap();
        assert_eq!(data["Name"], "Excalibur");
        assert_eq!(data["Health"], 100);
    }

    #[test]
    fn test_nested_table() {
        let data = eval_lua_module(
            r#"return { ["Excalibur"] = { Name = "Excalibur", Type = "Warframe", Health = 100, Sprint = 1.0 } }"#,
        ).unwrap();
        assert_eq!(data["Excalibur"]["Name"], "Excalibur");
        assert_eq!(data["Excalibur"]["Health"], 100);
        assert_eq!(data["Excalibur"]["Sprint"], 1.0);
    }

    #[test]
    fn test_array_field() {
        let data = eval_lua_module(
            r#"return { Abilities = {"Slash Dash", "Radial Blind", "Radial Javelin", "Exalted Blade"} }"#,
        ).unwrap();
        let abilities = data["Abilities"].as_array().unwrap();
        assert_eq!(abilities.len(), 4);
        assert_eq!(abilities[0], "Slash Dash");
    }

    #[test]
    fn test_nil_and_bool() {
        let data = eval_lua_module(r#"return { Vaulted = true, Missing = nil }"#).unwrap();
        assert_eq!(data["Vaulted"], true);
        assert!(data["Missing"].is_null());
    }

    #[test]
    fn test_empty_table() {
        let data = eval_lua_module(r#"return {}"#).unwrap();
        assert!(data.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_module_with_require() {
        // Modules that use require() should get an empty table back, not crash
        let data = eval_lua_module(
            r#"local utils = require("Module:Utils"); return { Name = "Test" }"#,
        ).unwrap();
        assert_eq!(data["Name"], "Test");
    }
}
