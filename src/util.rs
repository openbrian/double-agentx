use agentx::encodings::ID;

pub fn as_vec(value: &ID) -> Vec<u32> {
    value.to_string()
        .split('.')
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<u32>>()
}
