use agentx::encodings::ID;

pub fn as_vec(value: &ID) -> Vec<u32> {
    value.to_string()
        .split('.')
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<u32>>()
}


#[cfg(test)]
mod tests {
    use agentx::encodings;
    use super::*;

    #[test]
    fn it_works() {
        let vec = vec![1, 3, 6, 1, 2, 1, 1, 1, 0];
        let oid = encodings::ID::try_from(vec.clone())
            .expect("Failed to convert");
        let result = as_vec(&oid);
        assert_eq!(result, vec);
    }
}
