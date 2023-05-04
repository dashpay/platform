mod generate;

fn main() {
    print!("Generating protobuf definitions... ");

    crate::generate::generate().expect("protobuf generation failed");
    println!("DONE");
}
