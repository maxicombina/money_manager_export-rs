use rusqlite::{Connection, OpenFlags /*, Result*/};
use clap::Parser;

#[derive(Parser,Debug)]
#[command(name = "Money Manager Export")]
#[command(author = "Maxi Combina <maxicombina@gmail.com>")]
#[command(version)]
#[command(about = "Exports Money Manager transactions in a suitable format for later analysis", long_about = None)]
struct Args {
    /// The exported backup file from Money Manager
    file_name: String,
    
    /// Start date in format "YYYY-MM-DD". If not provided, the 1st day of last month is used
    #[arg(short, long)]
    start_date: Option<String>,

    /// End date in format "YYYY-MM-DD". If not provided, the last day of last month is used
    #[arg(short, long)]
    end_date: Option<String>,
    
    /// Process full month from current year. Accepted values are numeric or Jan/January/Ene/Enero, etc
    #[arg(short, long)]
    month: Option<String>,
    
    /// Increase program debug messages. Can be specified multiple times 
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: Option<u8>,
}

fn get_query_statement() -> String {
    let mut str_query = String::from("");
    str_query.push_str("SELECT z.zdate, z.ztxdatestr, c.zname, z.zcontent, z.zamount, a.znicname ");
    str_query.push_str("FROM ZASSET a, ZCATEGORY c, ZINOUTCOME z ");
    str_query.push_str("WHERE z.ztxdatestr ");
    str_query.push_str("BETWEEN \"2023-05-01\" AND \"2023-05-31\" ");
    str_query.push_str("AND z.zisdel = 0 "); // zisdel flags deleted entries
    str_query.push_str("AND z.zdo_type = 1 "); // Type 1 is "expenses")
    str_query.push_str("AND z.ZASSETUID = a.ZUID "); // Join asset (pay method))
    str_query.push_str("AND z.ZCATEGORYUID = c.ZUID "); // Join Category
    str_query.push_str("ORDER BY z.zdate ASC");

    str_query
}
fn main() {
    
    let args = Args::parse();
    println!("{:?}", args);
    return;
    let conn =
        Connection::open_with_flags("20230602_213618.mmbak", OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    
    let str_query = get_query_statement();
    let mut stmt = conn.prepare(&str_query).unwrap();
    let mut rows = stmt.query([]).unwrap();

    let mut tot_amt = 0.0;
    while let Some(row) = rows.next().unwrap() {
        //println!("{}", row.get_unwrap(0));
        let _cocoa_timestamp: f64 = row.get_unwrap(0); // skipable
        let date: String = row.get_unwrap(1);
        let category: String = row.get_unwrap(2);
        let name: String = row.get_unwrap(3);
        let amt: f32 = row.get_unwrap(4);
        let pay_method: String = row.get_unwrap(5);

        println!("{};{};{};{:.2};{}", date, category, name, amt, pay_method);
        tot_amt += amt;
    }

    println!("total amount: {:.2}", tot_amt);
    //    conn.close();
}
