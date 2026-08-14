use dotenvy::dotenv;
use reqwest::blocking::Client;
use serde_json::json;
use std::env;
use teloxide::utils::command::BotCommands;
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos disponíveis:")]
enum ComandosTelegram {
    #[command(description = "Inicia a conversa")]
    Start,

    #[command(description = "Mostra ajuda")]
    Help,

    #[command(description = "Configura o perfil (ex: /config tema dark)")]
    Config(String), // Você pode capturar argumentos na mesma linha!
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
    let out = String::new();
    let url = format!("https://api.smartthings.com/v1/devices/{}/commands", id);
    let client = Client::new();
    if let ComandosAr::Status = command {
        let url = format!("https://api.smartthings.com/v1/devices/{}/status", id);
        let response = client.get(&url).bearer_auth(api).send()?;

        let status = response.status();
        let body_text = response.text()?;

        if !status.is_success() {
            return Err(
                format!("Erro da API SmartThings (Status {}): {}", status, body_text).into(),
            );
        }

        return Ok(body_text);
    }
    let argument = match command {
        ComandosAr::Switch(mode) => {
            let state = if *mode { "on" } else { "off" };
            json!({
                "commands": [
                    {
                        "component": "main",
                        "capability": "switch",
                        "command": state,
                        "arguments": [] // Alguns firmwares do SmartThings exigem o array arguments vazio para o switch
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

    // Guarda o status code para verificar depois
    let status = response.status();
    let body_text = response.text()?;

    // Se não deu sucesso, imprime o corpo do erro que a Samsung devolveu
    if !status.is_success() {
        println!("ERRO DA API ({}): {}", status, body_text);
    } else {
        println!("Sucesso: {}", body_text);
    }

    Ok(out)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let ar_id = env::var("AR_QUARTO").expect("no device id on .env");
    let api_samsung = env::var("SMART_THINGS_TOKEN").expect("no api key for smart things");
    println!(
        "{:#?}",
        call_api_samsung(&ar_id, &api_samsung, &ComandosAr::Temp(22)).unwrap()
    );
    Ok(())
}
