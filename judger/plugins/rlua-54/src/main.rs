use std::process::ExitCode;

mod compile;
mod execute;
mod violate;
const LUA_SRC: &str = "/src/code.txt";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let cmd=args.get(1).unwrap().as_str();

    match cmd {
        "compile" => compile::compile(),
        "execute" => execute::execute(),
        "violate" => match args.get(2).unwrap().as_str(){
            "cpu" => violate::cpu(),
            "mem" => violate::mem(),
            "disk" => violate::disk(),
            "net" => violate::net(),
            "syscall" => violate::syscall(),
            _ => println!("3: Invalid command"),
        },
        "hello" => println!("hello world"),
        _ => println!("4: Invalid command: \"{}\"", cmd),
    };

    ExitCode::from(0)
}
