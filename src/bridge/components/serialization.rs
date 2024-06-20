pub trait Serialization {
    fn deserialize(&self, data: &str) -> Self;

    fn serialize(&self) -> &str {
        let data = ">>>>>>>>>>>>>>>>>>>> object serialized";
        println!("{}", data);
        data
    }
}
