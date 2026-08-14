use dotenvy::dotenv;
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::env::{self, args};
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos disponíveis:")]
enum ComandosTelegram {
    #[command(description = "Inicia a conversa")]
    Start,

    #[command(description = "Mostra ajuda")]
    Help,

    #[command(description = "Configura o perfil (ex: /config tema dark)")]
    Config(String),
}

#[derive(Debug, Clone, Copy)]
pub enum Modes {
    Auto,
    Cool,
    Dry,
    Wind,
}

impl Modes {
    pub fn as_str(&self) -> &'static str {
        match self {
            Modes::Auto => "auto",
            Modes::Cool => "cool",
            Modes::Dry => "dry",
            Modes::Wind => "wind",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Fans {
    Fixed,
    Vertical,
    Horizontal,
    All,
}

impl Fans {
    pub fn as_str(&self) -> &'static str {
        match self {
            Fans::Fixed => "fixed",
            Fans::Vertical => "vertical",
            Fans::Horizontal => "horizontal",
            Fans::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ComandosAr {
    Status,
    Switch(bool),
    Mode(Modes),
    Temp(u8),
    Fan(Fans),
}

fn call_api_samsung(
    id: &String,
    api: &String,
    command: &ComandosAr,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    if let ComandosAr::Status = command {
        let url = format!("https://api.smartthings.com/v1/devices/{}/status", id);
        let response = client
            .get(&url)
            .bearer_auth(api)
            .send()?
            .error_for_status()?;

        let status = response.status();
        let body_text = response.text()?;

        if !status.is_success() {
            return Err(
                format!("Erro da API SmartThings (Status {}): {}", status, body_text).into(),
            );
        }

        // Parse do JSON bruto para extrair de forma limpa as informações essenciais
        let json_val: Value = serde_json::from_str(&body_text)?;
        let main = json_val.pointer("/components/main");

        let power = main
            .and_then(|m| m.pointer("/switch/switch/value"))
            .and_then(|v| v.as_str())
            .unwrap_or("desconhecido");

        let power_icon = if power == "on" {
            "🟢 Ligado"
        } else {
            "🔴 Desligado"
        };

        let temp = main
            .and_then(|m| m.pointer("/temperatureMeasurement/temperature/value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let target_temp = main
            .and_then(|m| m.pointer("/thermostatCoolingSetpoint/coolingSetpoint/value"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let humidity = main
            .and_then(|m| m.pointer("/relativeHumidityMeasurement/humidity/value"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let mode = main
            .and_then(|m| m.pointer("/airConditionerMode/airConditionerMode/value"))
            .and_then(|v| v.as_str())
            .unwrap_or("desconhecido");

        let fan_mode = main
            .and_then(|m| m.pointer("/airConditionerFanMode/fanMode/value"))
            .and_then(|v| v.as_str())
            .unwrap_or("desconhecido");

        let resposta_bonita = format!(
            "❄️ *Status do Ar-Condicionado*\n\
             ────────────────────\n\
             🔌 *Estado:* `{}`\n\
             🌡️ *Temperatura Atual:* `{}°C`\n\
             🎯 *Setpoint (Alvo):* `{}°C`\n\
             💧 *Umidade:* `{}%`\n\
             🌀 *Modo:* `{}`\n\
             💨 *Ventilação:* `{}`",
            power_icon, temp, target_temp, humidity, mode, fan_mode
        );

        return Ok(resposta_bonita);
    }

    let url = format!("https://api.smartthings.com/v1/devices/{}/commands", id);
    let argument = match command {
        ComandosAr::Switch(mode) => {
            let state = if *mode { "on" } else { "off" };
            json!({
                "commands": [
                    {
                        "component": "main",
                        "capability": "switch",
                        "command": state,
                        "arguments": []
                    }
                ]
            })
        }
        ComandosAr::Mode(mode) => json!({
            "commands": [
                {
                    "component": "main",
                    "capability": "airConditionerMode",
                    "command": "setAirConditionerMode",
                    "arguments": [
                        mode.as_str()
                    ]
                }
            ]
        }),
        ComandosAr::Fan(fan) => json!({
            "commands": [
                {
                    "component": "main",
                    "capability": "fanOscillationMode",
                    "command": "setFanOscillationMode",
                    "arguments": [
                        fan.as_str()
                    ]
                }
            ]
        }),
        ComandosAr::Temp(temp) => {
            if *temp >= 16_u8 && *temp <= 30_u8 {
                json!({
                    "commands": [
                        {
                            "component": "main",
                            "capability": "thermostatCoolingSetpoint",
                            "command": "setCoolingSetpoint",
                            "arguments": [
                                temp
                            ]
                        }
                    ]
                })
            } else {
                return Err("Temperature must be beetween 16 and 30".into());
            }
        }
        _ => {
            return Err("command not identified".into());
        }
    };

    let response = client.post(&url).bearer_auth(api).json(&argument).send()?;

    let status = response.status();
    let body_text = response.text()?;

    if !status.is_success() {
        println!("ERRO DA API ({}): {}", status, body_text);
    } else {
        println!("Sucesso: {}", body_text);
    }

    Ok(body_text)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let ar_id = env::var("AR_QUARTO").expect("no device id on .env");
    let api_samsung = env::var("SMART_THINGS_TOKEN").expect("no api key for smart things");

    let arguments = args().collect::<Vec<String>>();
    if arguments.len() > 1 {
        match arguments[1].as_str() {
            "status" => {
                println!(
                    "{}",
                    call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Status).unwrap()
                );
            }
            "temp" => {
                if arguments.len() >= 3 {
                    let temp_val: u8 = arguments[2]
                        .parse()
                        .expect("Erro ao converter temperatura para número");
                    println!(
                        "{}",
                        call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Temp(temp_val))
                            .unwrap()
                    );
                } else {
                    panic!("Parâmetro de temperatura ausente (ex: cargo run temp 22)");
                }
            }
            "switch" => {
                if arguments.len() >= 3 {
                    let state_bool = match arguments[2].to_lowercase().as_str() {
                        "on" | "true" | "1" => true,
                        "off" | "false" | "0" => false,
                        _ => panic!("Estado inválido para switch. Use 'on' ou 'off'"),
                    };
                    println!(
                        "{}",
                        call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Switch(state_bool))
                            .unwrap()
                    );
                } else {
                    panic!("Parâmetro ausente para switch (ex: cargo run switch on)");
                }
            }
            "mode" => {
                if arguments.len() >= 3 {
                    let mode_enum = match arguments[2].to_lowercase().as_str() {
                        "auto" => Modes::Auto,
                        "cool" => Modes::Cool,
                        "dry" => Modes::Dry,
                        "wind" => Modes::Wind,
                        _ => panic!("Modo inválido. Use: auto, cool, dry ou wind"),
                    };
                    println!(
                        "{}",
                        call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Mode(mode_enum))
                            .unwrap()
                    );
                } else {
                    panic!("Parâmetro ausente para mode (ex: cargo run mode cool)");
                }
            }
            "fan" => {
                if arguments.len() >= 3 {
                    let fan_enum = match arguments[2].to_lowercase().as_str() {
                        "fixed" => Fans::Fixed,
                        "vertical" => Fans::Vertical,
                        "horizontal" => Fans::Horizontal,
                        "all" => Fans::All,
                        _ => panic!("Oscilação inválida. Use: fixed, vertical, horizontal ou all"),
                    };
                    println!(
                        "{}",
                        call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Fan(fan_enum)).unwrap()
                    );
                } else {
                    panic!("Parâmetro ausente para fan (ex: cargo run fan vertical)");
                }
            }
            other => {
                println!(
                    "Comando desconhecido: '{}'. Opções válidas: status, temp, switch, mode, fan",
                    other
                );
            }
        }
    }
    Ok(())
}
