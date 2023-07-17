use chrono::{Datelike, NaiveDate};
use clap::Parser;
use faccess::{AccessMode, PathExt};
use rusqlite::{Connection, OpenFlags /*, Result*/};
use std::collections::HashMap;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(name = "Money Manager Export")]
#[command(author = "Maxi Combina <maxicombina@gmail.com>")]
#[command(version)]
#[command(about = "Exports Money Manager transactions in a suitable format for later analysis", long_about = None)]
struct Args {
    /// The exported backup file from Money Manager
    file_name: String,

    /// Start date in format "YYYY-MM-DD". If not provided, the first day of last month is used
    #[arg(short, long)]
    start_date: Option<String>,

    /// End date in format "YYYY-MM-DD". If not provided, the last day of last month is used
    #[arg(short, long)]
    end_date: Option<String>,

    /// Process full month from current year. Accepted values are numeric or Jan/January/Ene/Enero, etc
    //#[arg(short, long, allow_negative_numbers = true)] --> trick to allow negative numbers in CLI options
    #[arg(short, long)]
    month: Option<String>,

    /// Increase program debug messages. Can be specified multiple times
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: Option<u8>,
}

// A processed version of Args. I don't want Option<T> all over the place.
#[derive(Debug, Default)]
struct Config {
    file_name: String,
    start_date: String,
    end_date: String,
    debug_level: u8,
}

// Based on https://stackoverflow.com/questions/53687045/how-to-get-the-number-of-days-in-a-month-in-rust,
// but using from_ymd_opt() rather than the deprecated from_ymd() in chrono-0.4.23
pub fn get_days_from_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(
        match month {
            12 => year + 1,
            _ => year,
        },
        match month {
            12 => 1,
            _ => month + 1,
        },
        1,
    )
    .expect("Date constructed must be valid")
    .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
    .num_days()
    .try_into()
    .expect("Converted an i64 into i32, but num_days() must always be <= 31")
}

fn get_query_statement() -> String {
    let mut str_query = String::from("");
    str_query.push_str("SELECT z.zdate, z.ztxdatestr, c.zname, z.zcontent, z.zamount, a.znicname ");
    str_query.push_str("FROM ZASSET a, ZCATEGORY c, ZINOUTCOME z ");
    str_query.push_str("WHERE z.ztxdatestr ");
    str_query.push_str("BETWEEN ?1 AND ?2 "); // Begin and end dates
    str_query.push_str("AND z.zisdel = 0 "); // zisdel flags deleted entries
    str_query.push_str("AND z.zdo_type = 1 "); // Type 1 is "expenses")
    str_query.push_str("AND z.ZASSETUID = a.ZUID "); // Join asset (pay method))
    str_query.push_str("AND z.ZCATEGORYUID = c.ZUID "); // Join Category
    str_query.push_str("ORDER BY z.zdate ASC");

    str_query
}

fn parse_month(month: &Option<String>) -> Option<u8> {
    let month_str: &String;

    if month.is_none() {
        //println!("No month in command line");
        return None;
    } else {
        month_str = month.as_ref().unwrap();
        //println!("Month in command line: {}", month_str);
    }

    let mut months = HashMap::new();
    months.insert("jan", 1);
    months.insert("january", 1);
    months.insert("ene", 1);
    months.insert("enero", 1);
    months.insert("feb", 2);
    months.insert("february", 2);
    months.insert("febrero", 2);
    months.insert("mar", 3);
    months.insert("march", 3);
    months.insert("marzo", 3);
    months.insert("apr", 4);
    months.insert("april", 4);
    months.insert("abr", 4);
    months.insert("abril", 4);
    months.insert("may", 5);
    months.insert("mayo", 5);
    months.insert("jun", 6);
    months.insert("june", 6);
    months.insert("junio", 6);
    months.insert("jul", 7);
    months.insert("july", 7);
    months.insert("julio", 7);
    months.insert("aug", 8);
    months.insert("august", 8);
    months.insert("ago", 8);
    months.insert("agosto", 8);
    months.insert("sep", 9);
    months.insert("september", 9);
    months.insert("septiembre", 9);
    months.insert("oct", 10);
    months.insert("october", 10);
    months.insert("octubre", 10);
    months.insert("nov", 11);
    months.insert("november", 11);
    months.insert("noviembre", 11);
    months.insert("dec", 12);
    months.insert("december", 12);
    months.insert("dic", 12);
    months.insert("diciembre", 12);

    //println!("{:#?}", months);
    // First, try to obtain the month from a string
    if months.contains_key(month_str.to_lowercase().as_str()) {
        let month_index = months
            .get(month_str.to_lowercase().as_str())
            .copied()
            .unwrap();
        return Some(month_index);
    }

    // Second, try to obtain a month from a number
    // Nice way to transform a Result<> into an Option<>
    month_str.parse::<u8>()
        .ok()
        .filter(|v| *v >= 1 && *v <= 12)
}
fn process_category(category: String) -> String {
    // As of this writing there seems to be no more 'category/sub-category', only 'category'
    category.trim().to_string()
}

// The description added to the transaction
fn process_name(name: String) -> String {
    name.trim().to_string()
}

// Transform float "x.y" into String "x,y".
fn process_amount(amount: f64) -> String {
    //let mut amt_str = amount.to_string();
    let integer_part = amount.floor().to_string();
    let decimal_part = format!("{:02}", (100.0 * amount.fract()).round());
    //println!("f32: {}, integer: {}, decimal: {}", amount, integer_part, decimal_part);

    let amt_str = integer_part + "," + &decimal_part;

    amt_str
}

fn process_date(date: String) -> String {
    let parts: Vec<&str> = date.rsplit("-").collect();
    parts.join("/")
}

fn process_payment_method(pay_method: String) -> String {
    let ret_pay_method: String;
    match pay_method.as_str() {
        "Tickets" => ret_pay_method = "Ti".to_string(),
        "Transferencia" => ret_pay_method = "T".to_string(),
        "Efectivo" => ret_pay_method = "E".to_string(),
        "T. Débito" => ret_pay_method = "TD".to_string(),
        "T. Crédito" => ret_pay_method = "TC".to_string(),
        "PayPal" => ret_pay_method = "P".to_string(),
        _ => ret_pay_method = "INVALID".to_string(),
    }
    ret_pay_method
}
fn query_and_print(config: &Config) {
    let conn =
        Connection::open_with_flags(&config.file_name, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let str_query = get_query_statement();
    //    println!("strquery: '{}'", &str_query);
    let mut stmt = conn.prepare(&str_query).unwrap();

    //    println!("USING start date: {}", &config.start_date);
    //    println!("USING end date: {}", &config.end_date);

    let mut rows = stmt.query([&config.start_date, &config.end_date]).unwrap();

    println!("fecha;categoría;comentario;importe;forma pago");
    let mut tot_amt: f64 = 0.0;
    while let Some(row) = rows.next().unwrap() {
        //println!("{}", row.get_unwrap(0));
        let _cocoa_timestamp: f64 = row.get_unwrap(0); // skip. Left here as a reminder of the data type ('cocoa timestamp')
        let date: String = process_date(row.get_unwrap(1));
        let category: String = process_category(row.get_unwrap(2));
        let name: String = process_name(row.get_unwrap(3));
        let amt: String = process_amount(row.get_unwrap(4));
        let pay_method: String = process_payment_method(row.get_unwrap(5));

        println!("{};{};{};{};{}", date, category, name, amt, pay_method);
        tot_amt += row.get_unwrap::<usize, f64>(4);
    }

    println!("Total: {:.2}", tot_amt);

    //    conn.close();
}

fn init_config(args: &Args, config: &mut Config) {
    // Non-date of config params
    config.file_name = args.file_name.clone();
    config.debug_level = args.debug.unwrap_or(0);

    // Basic check on database file
    let database_path = Path::new(&config.file_name);
    if !database_path.exists() || database_path.access(AccessMode::READ).is_err() {
        eprintln!("Cannot read database file '{}'", &config.file_name);
        std::process::exit(1);
    }
    // Date config params
    let month_opt = parse_month(&args.month);
    if month_opt.is_some() {
        //println!("Parsing month");
        // We have a month, let it take priority
        let end_day = get_days_from_month(chrono::Utc::now().year(), month_opt.unwrap().into());

        let start_date =
            NaiveDate::from_ymd_opt(chrono::Utc::now().year(), month_opt.unwrap().into(), 1)
                .unwrap();
        config.start_date = start_date.to_string().clone();

        let end_date = NaiveDate::from_ymd_opt(
            chrono::Utc::now().year(),
            month_opt.unwrap().into(),
            end_day,
        )
        .unwrap();
        config.end_date = end_date.to_string();

        return;
    }

    //println!("No month provided");
    /* No month provided, let's see start/end dates */
    let parsed_start_date;
    let parsed_end_date;
    if args.start_date.is_none() {
        // No month, no start date => use last month for start_date
        let start_date;
        if chrono::Utc::now().month() == 1 {
            start_date = NaiveDate::from_ymd_opt(chrono::Utc::now().year() - 1, 12, 1).unwrap();
        } else {
            start_date = NaiveDate::from_ymd_opt(
                chrono::Utc::now().year(),
                chrono::Utc::now().month() - 1,
                1,
            )
            .unwrap();
        }
        config.start_date = start_date.to_string();
    } else {
        config.start_date = args.start_date.as_ref().unwrap().clone();
    }

    // Check and format start date
    parsed_start_date = NaiveDate::parse_from_str(&config.start_date, "%Y-%m-%d");
    if parsed_start_date.is_err() {
        eprintln!(
            "Invalid start date provided: '{}'. Please use format YYYY-MM-DD and a valid date",
            &config.start_date
        );
        std::process::exit(1);
    }
    // This is done to transform '2023-2-1' into '2023-02-01', otherwise the query to sqlite does not work correctly.
    assert!(parsed_start_date.is_ok());
    config.start_date = parsed_start_date.unwrap().to_string();

    if args.end_date.is_none() {
        // No end date: use the last day of config.start_date (already set above)
        let end_date;

        assert!(parsed_start_date.is_ok());
        let num_days_in_month = get_days_from_month(
            parsed_start_date.unwrap().year(),
            parsed_start_date.unwrap().month(),
        );
        end_date = NaiveDate::from_ymd_opt(
            parsed_start_date.unwrap().year(),
            parsed_start_date.unwrap().month(),
            num_days_in_month,
        )
        .unwrap();
        config.end_date = end_date.to_string();
    } else {
        config.end_date = args.end_date.as_ref().unwrap().clone();
    }

    // Check and format end date
    parsed_end_date = NaiveDate::parse_from_str(&config.end_date, "%Y-%m-%d");
    if parsed_end_date.is_err() {
        eprintln!(
            "Invalid end date provided: '{}'. Please use format YYYY-MM-DD and a valid date",
            &config.end_date
        );
        std::process::exit(1);
    }
    // This is done to transform '2023-2-1' into '2023-02-01', otherwise the query to sqlite does not work correctly.
    assert!(parsed_end_date.is_ok());
    config.end_date = parsed_end_date.unwrap().to_string();

    ////println!("Start date: {}", args.start_date.as_ref().unwrap());
    ////println!("End date: {}", args.end_date.as_ref().unwrap());
}

fn main() {
    let args = Args::parse();
    let mut config = Default::default();
    //println!("Args: {:?}", args);
    //println!("{:?}", config);

    init_config(&args, &mut config);

    ////println!("Args: {:?}", args);
    //println!("Config: {:?}", config);
    query_and_print(&config);
}
