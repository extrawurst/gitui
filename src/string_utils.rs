///
pub fn trim_length_left(s: &str, width: usize) -> &str {
	let len = s.len();
	if len > width {
		for i in len - width..len {
			if s.is_char_boundary(i) {
				return &s[i..];
			}
		}
	}

	s
}

//TODO: allow customize tabsize
pub fn tabs_to_spaces(input: String) -> String {
	if input.contains('\t') {
		input.replace("\t", "  ")
	} else {
		input
	}
}

#[cfg(test)]
mod test {
	use pretty_assertions::assert_eq;

	use crate::string_utils::trim_length_left;

	#[test]
	fn test_trim() {
		assert_eq!(trim_length_left("👍foo", 3), "foo");
		assert_eq!(trim_length_left("👍foo", 4), "foo");
	}
}
