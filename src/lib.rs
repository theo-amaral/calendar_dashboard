use chrono::{DateTime, Local, TimeDelta};
use colored::{Color, Colorize};
use google_calendar::Client;
use google_calendar::types::{CalendarListEntry, Event, MinAccessRole, OrderBy};
use log::debug;
use reqwest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::Path;
use std::process;
use termion::terminal_size;
use unicode_width::UnicodeWidthStr;

const TOKEN_PATH: &str = "/home/theo/.config/calendar_dashboard/tokens.json";
const CREDENTIALS_PATH: &str = "/home/theo/.config/calendar_dashboard/client_secret.json";
const PORT: u16 = 8888;

#[allow(dead_code)]
#[derive(Deserialize)]
struct Installed {
    client_id: String,
    client_secret: String,
    redirect_uris: Vec<String>,
}

#[derive(Deserialize)]
struct ClientSecret {
    installed: Installed,
}

#[derive(Serialize, Deserialize, Clone)]
struct StoredToken {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Local>,
}

#[derive(Deserialize, Debug)]
pub struct TaskListResponse {
    pub items: Option<Vec<GoogleTask>>,
}

#[derive(Deserialize, Debug)]
pub struct GoogleTask {
    pub title: String,
    pub status: String,
}

fn load_tokens() -> Result<StoredToken, io::Error> {
    if !Path::new(TOKEN_PATH).exists() {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Arquivo não existe",
        ))
    } else {
        let data = fs::read_to_string(TOKEN_PATH)?;
        let tokens = serde_json::from_str(&data)?;
        Ok(tokens)
    }
}

fn store_tokens(tokens: &StoredToken) {
    let data = serde_json::to_string_pretty(tokens);
    fs::write(TOKEN_PATH, data.unwrap()).unwrap();
}

async fn perform_initial_login(client: &mut Client) -> StoredToken {
    println!("{}", "Autenticação necessária.".yellow().bold());

    let redirect_uri = format!("http://127.0.0.1:{}", PORT);
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).unwrap();

    let auth_url = client.user_consent_url(&[
        String::from("https://www.googleapis.com/auth/calendar.readonly"),
        String::from("https://www.googleapis.com/auth/tasks.readonly"),
    ]);

    println!("Abrindo navegador...");
    if webbrowser::open(&auth_url).is_err() {
        println!("Clique no link: {}", auth_url.underline());
    }

    let mut code = String::new();

    if let Some(stream) = listener.incoming().next() {
        let mut stream = stream.unwrap();
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();

        if reader.read_line(&mut request_line).is_ok() {
            if let Some(path_start) = request_line.find("GET /") {
                if let Some(path_end) = request_line[path_start..].find(" HTTP/") {
                    let path = &request_line[path_start + 4..path_start + path_end];
                    let full_url = format!("{}{}", redirect_uri, path);

                    if let Ok(parsed) = url::Url::parse(&full_url) {
                        if let Some(c) = parsed.query_pairs().find(|(k, _)| k == "code") {
                            code = c.1.to_string();

                            let msg = "HTTP/1.1 200 OK\r\n\r\n<html><body style='background:#222;color:#dfd;text-align:center;padding:50px;font-family:sans-serif;'><h1>Sucesso!</h1><p>Pode fechar esta janela.</p><script>window.close()</script></body></html>";
                            stream.write_all(msg.as_bytes()).ok();
                        }
                    }
                }
            }
        }
    }

    if code.is_empty() {
        panic!("Não foi possível capturar o código da URL.")
    }

    println!("Código capturado! Obtendo tokens...");

    let access_token = client
        .get_access_token(&code, "authorization_code")
        .await
        .expect("Falha ao validar o código com o Google.");

    let refresh_token = access_token.refresh_token;

    let tokens = StoredToken {
        refresh_token,
        access_token: access_token.access_token,
        expires_at: Local::now()
            .checked_add_signed(TimeDelta::new(access_token.expires_in, 0).unwrap())
            .unwrap(),
    };
    store_tokens(&tokens);
    println!("{}", "Login concluído!".green());
    tokens
}

pub async fn create_client() -> (Client, String) {
    debug!("Creating client...");
    let data = fs::read_to_string(CREDENTIALS_PATH).unwrap();
    let client_infos: ClientSecret = serde_json::from_str(&data).unwrap();

    let redirect_uri = format!("http://127.0.0.1:{}", PORT);

    let mut c = Client::new(
        &client_infos.installed.client_id,
        &client_infos.installed.client_secret,
        redirect_uri.clone(),
        "",
        "",
    );

    let mut tokens = match load_tokens() {
        Err(_) => {
            println!(
                "{}",
                "Token não iniciado. Iniciando login...".yellow().bold()
            );
            perform_initial_login(&mut c).await
        }
        Ok(t) => t,
    };

    let mut c = Client::new(
        &client_infos.installed.client_id,
        &client_infos.installed.client_secret,
        redirect_uri.clone(),
        &tokens.access_token,
        &tokens.refresh_token,
    );

    if tokens.expires_at.timestamp_millis() < Local::now().timestamp_millis() {
        debug!("Refreshing token...");
        tokens.access_token = c.refresh_access_token().await.unwrap().access_token;
        tokens.expires_at = Local::now()
            .checked_add_signed(TimeDelta::milliseconds(
                c.expires_in().await.unwrap().as_millis() as i64,
            ))
            .unwrap();
        store_tokens(&tokens);
        c = Client::new(
            &client_infos.installed.client_id,
            &client_infos.installed.client_secret,
            redirect_uri.clone(),
            &tokens.access_token,
            &tokens.refresh_token,
        );
    }

    debug!("Client created!");
    (c, tokens.access_token)
}

pub async fn fetch_calendars(client: &Client) -> Vec<CalendarListEntry> {
    debug!("Fetching calendars...");
    let c = client
        .calendar_list()
        .list_all(MinAccessRole::Noop, false, false)
        .await
        .unwrap()
        .body;
    debug!("Fetched calendars!");
    c
}

pub async fn fetch_events(
    client: &Client,
    calendar_id: &str,
    time_min: &str,
    time_max: &str,
) -> Vec<Event> {
    debug!("Fetching calendar {calendar_id}...");
    let e = client
        .events()
        .list(
            calendar_id,
            "",
            0,
            250,
            OrderBy::StartTime,
            "",
            &[],
            "",
            &[],
            false,
            false,
            true,
            time_max,
            time_min,
            "",
            "",
        )
        .await
        .unwrap_or_else(|err| {
            eprintln!("error: {err}");
            process::exit(1);
        })
        .body;
    debug!("Fetched calendar {calendar_id}!");
    e
}

pub async fn fetch_tasks(token: &str) -> Vec<String> {
    debug!("Fetching tasks...");
    let client = reqwest::Client::new();

    let due_max = chrono::Utc::now()
        .checked_add_days(chrono::Days::new(3))
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let url = format!(
        "https://tasks.googleapis.com/tasks/v1/lists/@default/tasks?showCompleted=false&dueMax={}",
        due_max
    );

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(text) = response.text().await {
                    if let Ok(task_list) = serde_json::from_str::<TaskListResponse>(&text) {
                        let tasks = task_list.items.unwrap_or_default();
                        debug!("Fetched {} tasks!", tasks.len());
                        return tasks.into_iter().map(|t| t.title).collect();
                    }
                }
            } else {
                eprintln!("Erro ao buscar tasks: {:?}", response.status());
            }
        }
        Err(e) => eprintln!("Erro na requisição das tasks: {}", e),
    }
    vec![]
}

pub fn draw_box_title(title: &str) {
    let (w, _) = terminal_size().unwrap();
    let w = w as usize;
    let title_part = format!("─── {} ", title);
    let remaining = w - (title_part.chars().count() + 2);
    let mut line = String::with_capacity(w);
    line.push('╭');
    line.push_str(&title_part);
    line.extend(std::iter::repeat_n('─', remaining));
    line.push('╮');
    println!("{line}");
}

pub fn draw_box_first_line(events: Vec<String>) {
    let (w, _) = terminal_size().unwrap();
    let w = w as usize;
    let title = "    Eventos de hoje: ";
    let base_width = 5 + title.chars().count();
    let display_quantity = events
        .iter()
        .scan(0, |acc, e| {
            *acc += visual_width(e);
            Some(*acc)
        })
        .position(|count| base_width + count > w)
        .unwrap_or(events.len());

    let mut line = String::with_capacity(w);
    line.push('│');
    line.push_str(title);

    // Ajuste de cores para o azul-petróleo (Teal/Dark Cyan) do print original:
    // Texto em branco-gelo e fundo azul-petróleo escuro
    let text_color = Color::TrueColor {
        r: 228,
        g: 253,
        b: 225,
    };
    let bg_color = Color::TrueColor {
        r: 2,
        g: 120,
        b: 135,
    }; // <--- Azul-petróleo mais saturado do print

    if events.is_empty() {
        line.push_str(&format!(
            "{}{}{}",
            "".color(bg_color),
            " Sem eventos hoje! ".color(text_color).on_color(bg_color),
            "".color(bg_color),
        ));
    } else {
        for event in events.iter().take(display_quantity) {
            line.push(' ');
            line.push_str(event);
            line.push(' ');
        }
    }

    let current_width = visual_width(&line);
    line.extend(std::iter::repeat_n(' ', w - 6 - current_width));

    if display_quantity < events.len() {
        line.push_str(&format!(
            " {} ",
            format!("+{:2}", events.len() - display_quantity)
                .color(Color::TrueColor {
                    r: 243,
                    g: 66,
                    b: 19
                })
                .on_color(Color::TrueColor { r: 105, g: 5, b: 0 }),
        ))
    } else {
        line.push_str("     ");
    }
    line.push('│');
    println!("{}", line);
}

pub fn draw_box_second_line(tasks: Vec<String>) {
    let (w, _) = terminal_size().unwrap();
    let w = w as usize;
    let title = "    Tarefas dos próximos 3 dias: ";
    let base_width = 5 + title.chars().count();
    let display_quantity = tasks
        .iter()
        .scan(0, |acc, t| {
            *acc += visual_width(t);
            Some(*acc)
        })
        .position(|count| base_width + count > w)
        .unwrap_or(tasks.len());

    let mut line = String::with_capacity(w);
    line.push('│');
    line.push_str(title);

    // Ajuste de cores para o tom de Amarelo-Amendoado/Oliva-Claro do print original:
    // Texto em cinza-escuro/azul-marinho e fundo amarelado claro
    let text_color = Color::TrueColor {
        r: 26,
        g: 36,
        b: 43,
    }; // <--- Texto escuro para dar contraste
    let bg_color = Color::TrueColor {
        r: 224,
        g: 224,
        b: 146,
    }; // <--- Amarelo pastel desbotado

    if tasks.is_empty() {
        line.push_str(&format!(
            "{}{}{}",
            "".color(bg_color),
            " Sem tarefas! ".color(text_color).on_color(bg_color),
            "".color(bg_color),
        ));
    } else {
        for task in tasks.iter().take(display_quantity) {
            line.push_str(&format!(
                "{}{}{}",
                "".color(bg_color),
                format!(" {} ", task).color(text_color).on_color(bg_color),
                "".color(bg_color)
            ));
            line.push(' ');
        }
    }

    let current_width = visual_width(&line);
    if w > 6 + current_width {
        line.extend(std::iter::repeat_n(' ', w - 6 - current_width));
    }

    if display_quantity < tasks.len() {
        line.push_str(&format!(
            " {} ",
            format!("+{:2}", tasks.len() - display_quantity)
                .color(Color::White)
                .on_color(Color::Red),
        ))
    } else {
        line.push_str("     ");
    }
    line.push('│');
    println!("{}", line);
}

pub fn draw_box_last_line() {
    let (w, _) = terminal_size().unwrap();
    let w = w as usize;
    let mut line = String::with_capacity(w);
    line.push('╰');
    line.extend(std::iter::repeat_n('─', w - 2));
    line.push('╯');
    println!("{}", line);
}

pub fn visual_width(s: &str) -> usize {
    let stripped = strip_ansi_escapes::strip(s);
    let clean = std::str::from_utf8(&stripped).unwrap_or("");
    UnicodeWidthStr::width(clean)
}
