use arg_enum::ArgEnum;
use structopt::StructOpt;

// Deriving ArgEnum will impl FromStr and Display (and therefore ToString)
#[derive(ArgEnum, Clone, Copy, Debug)]
// Same as the Serde attribute (literally, I copied the source code)
#[arg_enum(rename_all = "snake_case")]
enum Arg {
    // Adding this to a variant will impl Default
    #[arg_enum(default)]
    SomeDefault,

    // Same as the Serde attribute, again
    #[arg_enum(rename = "foo_in_disguise")]
    Foo,
    FooBar,
}

#[derive(StructOpt)]
struct Opt {
    // These structopt attributes cannot be automated by the macro, so it's still a little boilerplatey
    #[structopt(default_value, possible_values(Arg::possible_values()))]
    arg: Arg,
}

fn main() {
    let opt = Opt::from_args();

    println!("{:?}", opt.arg);
}
