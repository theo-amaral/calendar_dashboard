use calendar_dashboard::*;
use chrono::{DateTime, Days, Local, Timelike};
use colored::{Color, Colorize};
use google_calendar::types::Event;
use log::debug;
use std::sync::Arc;
use tokio::task::JoinHandle;

struct Calendar {
    events: Vec<Event>,
    index: usize,
}

struct CalendarFuture {
    events_future: JoinHandle<Vec<Event>>,
    index: usize,
}

#[tokio::main]
async fn main() {
    debug!("Starting program...");
    env_logger::init();
    do_call().await;
    debug!("Finished program!");
}

async fn do_call() {
    let (client_struct, token) = create_client().await;
    let client = Arc::new(client_struct);

    let time_min = Local::now().to_rfc3339();
    let time_max = Local::now()
        .checked_add_days(Days::new(1))
        .unwrap()
        .to_rfc3339();

    let mut calendars_future = vec![];
    for (i, calendar) in fetch_calendars(&client).await.iter().enumerate() {
        let client = Arc::clone(&client);
        let calendar_id = calendar.id.clone();
        let time_min = time_min.clone();
        let time_max = time_max.clone();
        calendars_future.push(CalendarFuture {
            events_future: tokio::spawn(async move {
                fetch_events(&client, &calendar_id, &time_min, &time_max).await
            }),
            index: i,
        })
    }

    let mut calendars = vec![];
    for calendar_future in calendars_future {
        calendars.push(Calendar {
            events: calendar_future.events_future.await.unwrap(),
            index: calendar_future.index,
        })
    }

    let print_colors = [
        (
            Color::TrueColor {
                r: 228,
                g: 253,
                b: 225,
            },
            Color::TrueColor {
                r: 2,
                g: 128,
                b: 144,
            },
        ),
        (
            Color::TrueColor {
                r: 142,
                g: 196,
                b: 110,
            },
            Color::TrueColor {
                r: 34,
                g: 68,
                b: 49,
            },
        ),
    ];

    let mut events = vec![];
    for calendar in calendars {
        for event in calendar.events {
            let i = calendar.index % print_colors.len();
            events.push((event.clone(), i));
        }
    }

    events.sort_by_key(|e| {
        if let Some(dt) = &e.0.start.as_ref().unwrap().date_time {
            dt.timestamp_millis()
        } else if let Some(d) = &e.0.start.as_ref().unwrap().date {
            d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis()
        } else {
            i64::MAX
        }
    });

    let mut event_strings = vec![];
    for (event, i) in events {
        if event.start.clone().unwrap().date.is_none() {
            let start: DateTime<Local> =
                DateTime::from(event.start.clone().unwrap().date_time.unwrap());
            let end: DateTime<Local> =
                DateTime::from(event.end.clone().unwrap().date_time.unwrap());
            event_strings.push(format!(
                "{}{}{}{}",
                "".color(print_colors[i].1),
                format!("{} ", event.summary)
                    .color(print_colors[i].0)
                    .on_color(print_colors[i].1),
                format!(" {} às {} ", start.format("%Hh%M"), end.format("%Hh%M"))
                    .color(print_colors[i].1)
                    .on_color(print_colors[i].0),
                "".color(print_colors[i].0)
            ));
        } else {
            event_strings.push(format!(
                "{}{}{}{}",
                "".color(print_colors[i].1),
                format!("{} ", event.summary)
                    .color(print_colors[i].0)
                    .on_color(print_colors[i].1),
                " Dia Todo "
                    .color(print_colors[i].1)
                    .on_color(print_colors[i].0),
                "".color(print_colors[i].0)
            ));
        }
    }

    let tasks_strings = fetch_tasks(&token).await;

    // change your name here
    let greeting = if Local::now().hour() < 12 {
        "Bom dia, Theo."
    } else if Local::now().hour() > 18 {
        "Boa noite, Theo."
    } else {
        "Boa tarde, Theo."
    };

    draw_box_title(greeting);
    draw_box_first_line(event_strings);
    draw_box_second_line(tasks_strings);
    draw_box_last_line();
}
