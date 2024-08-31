use std::cell::Cell;
use std::env;
use std::io;
use std::process;

//TODO : wildcards inside other patterns can continue to infinty

#[derive(Debug, Clone, PartialEq)]
enum PatternUnit {
    Digit,
    AlphaNumeric,
    PlainText(String),
    CharGroup(String),
    NegCharGroup(String),
    OneOrMore(Box<PatternUnit>),
    ZeroOrMore(Box<PatternUnit>),
    ZeroOrOne(Box<PatternUnit>),
    Wildcard,
    Alternation((Box<PatternUnit>, Box<PatternUnit>)),
    CapturingGroup(Vec<PatternUnit>),
    NonCapturingGroup(Vec<PatternUnit>),
    BackReference(u16),
}

fn ending_pattern_last_spacing(pattern_unit: &PatternUnit) -> u32 {
    let mut length: u32 = 0;

    match *pattern_unit {
        PatternUnit::AlphaNumeric => {
            length = 1;
        }
        PatternUnit::Digit => {
            length = 1;
        }
        PatternUnit::CharGroup(ref text) => {
            length = 1;
        }
        PatternUnit::NegCharGroup(ref text) => {
            length = 1;
        }
        PatternUnit::PlainText(ref text) => {
            length = (text.len() - 1) as u32;
        }
        PatternUnit::Wildcard => {
            length = 1;
        }
        PatternUnit::OneOrMore(ref pu) => {
            length = ending_pattern_last_spacing(pu);
        }

        PatternUnit::ZeroOrMore(ref pu) => {
            length = ending_pattern_last_spacing(pu);
        }

        PatternUnit::ZeroOrOne(ref pu) => {
            length = ending_pattern_last_spacing(pu);
        }
        PatternUnit::BackReference(val) => {
            length = 1;
        }
        PatternUnit::Alternation((ref pu1, ref pu2)) => {
            let val1 = ending_pattern_last_spacing(pu1);
            let val2 = ending_pattern_last_spacing(pu2);

            if val1 >= val2 {
                length = val1;
            } else {
                length = val2;
            }
        }

        PatternUnit::CapturingGroup(ref puvec) => {
            for i in 0..puvec.len() {
                length += ending_pattern_last_spacing(&puvec[i]);
            }
        }

        PatternUnit::NonCapturingGroup(ref puvec) => {
            for i in 0..puvec.len() {
                length += ending_pattern_last_spacing(&puvec[i]);
            }
        }
    }

    length
}

fn match_unitary_pattern(
    is_a_match: &mut bool,
    str_cursor: &mut usize,
    pattern_unit: &PatternUnit,
    input_line: &str,
    is_starting_pattern: bool,
    is_ending_pattern: bool,
    backreferences: &mut Vec<String>,
    current_group_index: &mut usize,
    shd_increase_current_group_index: &mut bool,
    next_pattern: Option<PatternUnit>,
) {
    match pattern_unit {
        PatternUnit::Digit => {
            if !('0'..='9')
                .collect::<Vec<char>>()
                .as_slice()
                .contains(&input_line.chars().nth(*str_cursor).unwrap())
            {
                *is_a_match = false;
            } else {
                *str_cursor += 1;
            }
        }

        PatternUnit::AlphaNumeric => {
            if !(('0'..='9')
                .collect::<Vec<char>>()
                .as_slice()
                .contains(&input_line.chars().nth(*str_cursor).unwrap())
                || ('a'..='z')
                    .collect::<Vec<char>>()
                    .as_slice()
                    .contains(&input_line.chars().nth(*str_cursor).unwrap())
                || ('A'..='Z')
                    .collect::<Vec<char>>()
                    .as_slice()
                    .contains(&input_line.chars().nth(*str_cursor).unwrap())
                || input_line.chars().nth(*str_cursor).unwrap() == '_')
            {
                *is_a_match = false;
            } else {
                *str_cursor += 1;
            }
        }

        PatternUnit::PlainText(plain_text) => {
            if is_starting_pattern {
                if !input_line.starts_with(plain_text) {
                    *is_a_match = false;
                } else {
                    *str_cursor += plain_text.len(); //got this out of if put it in else
                }
            } else if is_ending_pattern {
                if !input_line.ends_with(plain_text) {
                    *is_a_match = false;
                } else {
                    *str_cursor += plain_text.len(); //got this out of if put it in else
                }
            } else if *str_cursor + plain_text.len() <= input_line.len() {
                let ptxt = &input_line[*str_cursor..*str_cursor + plain_text.len()];

                if !(ptxt == plain_text) {
                    *is_a_match = false;
                } else {
                    *str_cursor += plain_text.len();
                } //should be an else
            } else {
                *is_a_match = false;
            }
        }

        PatternUnit::CharGroup(plain_text) => {
            if !plain_text
                .chars()
                .collect::<Vec<char>>()
                .contains(&input_line.chars().nth(*str_cursor).unwrap())
            {
                *is_a_match = false;
            } else {
                *str_cursor += 1;
            }
        }

        PatternUnit::NegCharGroup(plain_text) => {
            if plain_text
                .chars()
                .collect::<Vec<char>>()
                .contains(&input_line.chars().nth(*str_cursor).unwrap())
            {
                *is_a_match = false;
            } else {
                *str_cursor += 1;
            }
        }

        PatternUnit::OneOrMore(pu) => {
            let mut is_local_match: bool = true;

            match_unitary_pattern(
                &mut is_local_match,
                str_cursor,
                &pu,
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );

            if *str_cursor <= input_line.len() - 1 {
                match **pu {
                    PatternUnit::NegCharGroup(ref _ha) => {
                        if *str_cursor + 1 > input_line.len() - 1 {
                            *is_a_match = is_local_match;
                        }
                    }
                    _ => {}
                }

                if !is_local_match {
                    *is_a_match = is_local_match;
                }

                let mut is_match_temp: bool = false;
                match **pu {
                    PatternUnit::NegCharGroup(ref _ha) => {
                        is_match_temp = true;

                        let lpu = next_pattern.clone().unwrap();
                        let mut str_cursor_temp = str_cursor.clone();

                        match_unitary_pattern(
                            &mut is_match_temp,
                            &mut str_cursor_temp,
                            &lpu,
                            &input_line,
                            false,
                            false,
                            backreferences,
                            current_group_index,
                            &mut false,
                            next_pattern.clone(),
                        );
                    }
                    _ => {}
                }

                if !is_match_temp {
                    while is_local_match {
                        match **pu {
                            PatternUnit::NegCharGroup(ref _ha) => {
                                is_match_temp = true;

                                let lpu = next_pattern.clone().unwrap();
                                let mut str_cursor_temp = str_cursor.clone();

                                match_unitary_pattern(
                                    &mut is_match_temp,
                                    &mut str_cursor_temp,
                                    &lpu,
                                    &input_line,
                                    false,
                                    false,
                                    backreferences,
                                    current_group_index,
                                    &mut false,
                                    next_pattern.clone(),
                                );
                            }
                            _ => {}
                        }

                        if is_match_temp {
                            break;
                        }

                        match_unitary_pattern(
                            &mut is_local_match,
                            str_cursor,
                            &pu,
                            &input_line,
                            is_starting_pattern,
                            is_ending_pattern,
                            backreferences,
                            current_group_index,
                            &mut false,
                            next_pattern.clone(),
                        );

                        if *str_cursor > input_line.len() - 1 {
                            break;
                        }

                        match **pu {
                            PatternUnit::NegCharGroup(ref _ha) => {
                                if *str_cursor + 1 > input_line.len() - 1 {
                                    *is_a_match = is_local_match;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        PatternUnit::ZeroOrMore(pu) => {
            let mut is_local_match: bool = true;

            match_unitary_pattern(
                &mut is_local_match,
                str_cursor,
                &pu,
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );

            if *str_cursor <= input_line.len() - 1 {
                match **pu {
                    PatternUnit::NegCharGroup(ref _ha) => {
                        if *str_cursor + 1 > input_line.len() - 1 {
                            *is_a_match = is_local_match;
                        }
                    }
                    _ => {}
                }
                while is_local_match {
                    match_unitary_pattern(
                        &mut is_local_match,
                        str_cursor,
                        &pu,
                        &input_line,
                        is_starting_pattern,
                        is_ending_pattern,
                        backreferences,
                        current_group_index,
                        &mut false,
                        next_pattern.clone(),
                    );

                    if *str_cursor > input_line.len() - 1 {
                        break;
                    }

                    match **pu {
                        PatternUnit::NegCharGroup(ref _ha) => {
                            if *str_cursor + 1 > input_line.len() - 1 {
                                *is_a_match = is_local_match;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        PatternUnit::ZeroOrOne(pu) => {
            let mut is_local_match: bool = true;

            match_unitary_pattern(
                &mut is_local_match,
                str_cursor,
                &pu,
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );
        }

        PatternUnit::Wildcard => {
            *str_cursor += 1;
        }

        PatternUnit::Alternation((pu1, pu2)) => {
            let mut is_local_match1: bool = true;
            let mut is_local_match2: bool = true;

            let mut str_cursor_cpy1 = str_cursor.clone();
            let mut str_cursor_cpy2 = str_cursor.clone();

            match_unitary_pattern(
                &mut is_local_match1,
                &mut str_cursor_cpy1,
                &pu1,
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );

            match_unitary_pattern(
                &mut is_local_match2,
                &mut str_cursor_cpy2,
                &pu2,
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );

            if is_local_match1 {
                *str_cursor = str_cursor_cpy1;
            } else if is_local_match2 {
                *str_cursor = str_cursor_cpy2;
            }

            *is_a_match = is_local_match1 || is_local_match2;
        }

        PatternUnit::CapturingGroup(puvec) => {
            let mut is_local_match: bool = true;
            let mut backref_buffer: String = String::new();
            let str_cursor_start = *str_cursor;

            if *shd_increase_current_group_index {
                *current_group_index += 1;
            }

            let this_group_index = (*current_group_index).clone(); // we clone it so we get to preserve
                                                                   // it's value for this capgroup after multiple recussive calls

            if *str_cursor < input_line.len() {
                backref_buffer.push(input_line.chars().nth(*str_cursor).unwrap().clone());
            } else {
                is_local_match = false;
                *is_a_match = false;
            }

            if is_local_match {
                for i in 0..puvec.len() {
                    let local_next_pattern = if i + 1 <= puvec.len() - 1 {
                        Some(puvec[i + 1].clone())
                    } else {
                        next_pattern.clone()
                    };

                    is_local_match = true;

                    if *str_cursor < input_line.len() {
                        //backref_buffer
                        //    .push(input_line.chars().nth(str_cursor).unwrap().clone());
                    } else {
                        is_local_match = false;
                        *is_a_match = false;
                        break;
                    }

                    match_unitary_pattern(
                        &mut is_local_match,
                        str_cursor,
                        &puvec[i],
                        &input_line,
                        false,
                        false,
                        backreferences,
                        current_group_index,
                        shd_increase_current_group_index,
                        local_next_pattern.clone(),
                    );

                    if !is_local_match {
                        *is_a_match = false;
                        break;
                    }
                }
            }

            if is_local_match {
                backreferences[this_group_index - 1] =
                    input_line[str_cursor_start..*str_cursor].to_string();
                //backreferences.push(input_line[str_cursor_start..*str_cursor].to_string());
            } else {
            }
        }

        PatternUnit::NonCapturingGroup(puvec) => {
            let mut is_local_match: bool = true;
            let mut backref_buffer: String = String::new();

            if *str_cursor < input_line.len() {
                backref_buffer.push(input_line.chars().nth(*str_cursor).unwrap().clone());
            } else {
                is_local_match = false;
            }

            if is_local_match {
                for i in 0..puvec.len() {
                    let local_next_pattern = if i + 1 <= puvec.len() - 1 {
                        Some(puvec[i + 1].clone())
                    } else {
                        next_pattern.clone()
                    };

                    is_local_match = true;
                    match_unitary_pattern(
                        &mut is_local_match,
                        str_cursor,
                        &puvec[i],
                        &input_line,
                        false,
                        false,
                        backreferences,
                        current_group_index,
                        shd_increase_current_group_index,
                        local_next_pattern.clone(),
                    );

                    if !is_local_match {
                        *is_a_match = false;
                        break;
                    }
                }
            }
        }

        PatternUnit::BackReference(index) => {
            let mut is_local_match: bool = true;

            //TODO CHECK if backreferences.len() > ..
            match_unitary_pattern(
                &mut is_local_match,
                str_cursor,
                &PatternUnit::PlainText(backreferences[(*index - 1) as usize].clone()),
                &input_line,
                is_starting_pattern,
                is_ending_pattern,
                backreferences,
                current_group_index,
                shd_increase_current_group_index,
                next_pattern.clone(),
            );

            if !is_local_match {
                *is_a_match = false;
            }
        }
    }
}

struct CurrentAlternationInfo {
    group1: Cell<Vec<PatternUnit>>,
    group2: Cell<Vec<PatternUnit>>,
}

fn push_to_group(
    group: &mut PatternUnit,
    group_depth: i32,
    current_group_depth: i32,
    patten_to_psuh: &PatternUnit,
) {
    //recursive function to get the group at the correct depth and push to it
    match group {
        PatternUnit::CapturingGroup(children) => {
            let children_len = children.len();
            let mut i: u32 = 0;

            if current_group_depth == group_depth {
                children.push(patten_to_psuh.clone());
            } else {
                for child in children.iter_mut() {
                    if (i == (children_len - 1) as u32) && current_group_depth < group_depth {
                        push_to_group(child, group_depth, current_group_depth + 1, patten_to_psuh);
                    }
                    i += 1;
                }
            }
        }
        _ => {}
    }
}

fn first_alternation_group(
    group: &mut PatternUnit,
    group_depth: i32,
    current_group_depth: i32,
    patten_to_psuh: &PatternUnit,
    currrent_alternation_info: &mut CurrentAlternationInfo,
) {
    //recursive function to get the group at the correct depth and push to it
    match group {
        PatternUnit::CapturingGroup(children) => {
            let children_len = children.len();
            let mut i: u32 = 0;

            if current_group_depth == group_depth {
                //children.push(patten_to_psuh.clone());
                currrent_alternation_info.group1.set(children.clone());

                children.clear();
            } else {
                for child in children.iter_mut() {
                    if (i == (children_len - 1) as u32) && current_group_depth < group_depth {
                        first_alternation_group(
                            child,
                            group_depth,
                            current_group_depth + 1,
                            patten_to_psuh,
                            currrent_alternation_info,
                        );
                    }
                    i += 1;
                }
            }
        }
        _ => {}
    }
}

fn find_and_push_to_alternation_group(
    group: &mut PatternUnit, //pattern_parts_group
    group_depth: i32,        //pattern_parts_groups_stack_depth
    current_group_depth: i32,
    patten_to_psuh: &PatternUnit, //pattern_unit
    currrent_alternation_info: &mut CurrentAlternationInfo,
) {
    //recursive function to get the group at the correct depth and push to it
    match group {
        PatternUnit::CapturingGroup(children) => {
            let children_len = children.len();
            let mut i: u32 = 0;

            if current_group_depth == group_depth {
                children.push(patten_to_psuh.clone());

                /*add_pattern_unit(
                    is_alternation,
                    patten_to_psuh.clone(),
                    pattern_parts,
                    group_depth,
                    0,
                    &mut children[children_len - 1],
                    currrent_alternation_info,
                    first_alternation,
                ); */
            } else {
                for child in children.iter_mut() {
                    if (i == (children_len - 1) as u32) && current_group_depth < group_depth {
                        find_and_push_to_alternation_group(
                            child,
                            group_depth,
                            current_group_depth + 1,
                            patten_to_psuh,
                            currrent_alternation_info,
                        );
                    }
                    i += 1;
                }
            }
        }
        _ => {}
    }
}

fn add_child_groupto_group(group: &mut PatternUnit, group_depth: i32, current_group_depth: i32) {
    //recursive function to get the group at the correct depth and push to it
    match group {
        PatternUnit::CapturingGroup(children) => {
            let children_len = children.len();
            let mut i: u32 = 0;

            if current_group_depth == group_depth {
                children.push(PatternUnit::CapturingGroup(vec![]));
            } else {
                for child in children.iter_mut() {
                    if (i == (children_len - 1) as u32) && current_group_depth < group_depth {
                        add_child_groupto_group(child, group_depth, current_group_depth + 1);
                    }
                    i += 1;
                }
            }
        }
        _ => {}
    }
}

fn add_pattern_unit(
    is_alternation: &mut bool,
    pattern_unit: PatternUnit,
    pattern_parts: &mut Vec<PatternUnit>,
    pattern_parts_groups_stack_depth: i32,
    pattern_parts_groups_stack_alternation_depth: i32, //not needed, to be removed
    pattern_parts_group: &mut PatternUnit,
    currrent_alternation_info: &mut CurrentAlternationInfo,
    first_alternation: &mut bool,
) {
    if pattern_parts_groups_stack_depth == -1 {
        if !*is_alternation {
            pattern_parts.push(pattern_unit);
        } else {
            if *first_alternation {
                currrent_alternation_info.group1.set(pattern_parts.clone());
                pattern_parts.clear();

                currrent_alternation_info
                    .group2
                    .get_mut()
                    .push(pattern_unit);

                *first_alternation = false;
            } else {
                currrent_alternation_info
                    .group2
                    .get_mut()
                    .push(pattern_unit);
            }
        }
    } else {
        if !*is_alternation {
            push_to_group(
                pattern_parts_group,
                pattern_parts_groups_stack_depth,
                0,
                &pattern_unit,
            );
        } else {
            if *first_alternation {
                first_alternation_group(
                    pattern_parts_group,
                    pattern_parts_groups_stack_depth,
                    0,
                    &pattern_unit,
                    currrent_alternation_info,
                );

                //Get the old group formation put a clone of it in alterantion struct and clear it

                currrent_alternation_info
                    .group2
                    .get_mut()
                    .push(pattern_unit);

                *first_alternation = false;
            } else {
                currrent_alternation_info
                    .group2
                    .get_mut()
                    .push(pattern_unit);
            }
        }
    }
}

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        if pattern.chars().nth(0).unwrap() == '.' {
            if input_line.len() > (0 as usize) {
                return true;
            } else {
                return false;
            }
        }
        return input_line.contains(pattern);
    } else {
        let mut starting_pattern: Box<PatternUnit> = Box::new(PatternUnit::Digit);
        let mut ending_pattern: Box<PatternUnit> = Box::new(PatternUnit::Digit);
        let mut pattern_parts: Vec<PatternUnit> = vec![];
        let mut text_buffer: String = String::new();
        let mut k: usize = 0;
        let mut has_starting_pattern = false;
        let mut has_ending_pattern = false;

        //let mut pattern_parts_groups: Vec<Vec<PatternUnit>> = vec![];
        let mut pattern_parts_group: PatternUnit = PatternUnit::CapturingGroup(vec![]);
        let mut pattern_parts_groups_stack_depth: i32 = -1;
        let mut pattern_parts_groups_stack_alternation_depth: i32 = -1;

        let mut backreferences: Vec<String> = vec![];
        let mut init_backreferences: Vec<String> = vec![];

        let mut currrent_alternation_info: CurrentAlternationInfo = CurrentAlternationInfo {
            group1: Cell::new(vec![]),
            group2: Cell::new(vec![]),
        };

        currrent_alternation_info.group1.set(vec![]);
        currrent_alternation_info.group2.set(vec![]);

        if pattern.starts_with('^') {
            has_starting_pattern = true;
            k += 1;
        }

        if pattern.ends_with('$') {
            has_ending_pattern = true;
        }

        let mut is_alternation: bool = false;
        let mut is_first_alternation: bool = false;

        while k < pattern.chars().count() {
            if has_ending_pattern {
                if k == pattern.chars().count() - 1 {
                    if is_alternation {
                        is_alternation = false;

                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::Alternation((
                                Box::new(PatternUnit::NonCapturingGroup(
                                    currrent_alternation_info.group1.get_mut().clone(),
                                )),
                                Box::new(PatternUnit::NonCapturingGroup(
                                    currrent_alternation_info.group2.get_mut().clone(),
                                )),
                            )),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );

                        currrent_alternation_info.group1.get_mut().clear();
                        currrent_alternation_info.group2.get_mut().clear();
                    }

                    break;
                }
            }

            //
            //TODO Alternation inside character groups []
            if pattern.chars().nth(k).unwrap() == '\\' {
                if text_buffer != "" {
                    /*pattern_parts.push(PatternUnit::PlainText(text_buffer));*/

                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    text_buffer = String::new();
                }

                if pattern.chars().nth(k + 1).unwrap() == 'd' {
                    let mut plus_found = false;
                    let mut asteriks_found = false;
                    let mut question_mark_found = false;

                    if pattern.chars().count() > k + 1 + 1 {
                        //.count() - 1 taken to other side
                        if pattern.chars().nth(k + 2).unwrap() == '+' {
                            plus_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::OneOrMore(Box::new(PatternUnit::Digit)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts
                            //  .push(PatternUnit::OneOrMore(Box::new(PatternUnit::Digit)));
                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '*' {
                            asteriks_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrMore(Box::new(PatternUnit::Digit)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts
                            //.push(PatternUnit::ZeroOrMore(Box::new(PatternUnit::Digit)));
                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '?' {
                            question_mark_found = true;

                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrOne(Box::new(PatternUnit::Digit)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        }
                    }

                    if !plus_found && !asteriks_found && !question_mark_found {
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::Digit,
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                        //pattern_parts.push(PatternUnit::Digit);
                    }

                    k += 1;
                    k += 1;
                    continue;
                }

                if pattern.chars().nth(k + 1).unwrap() == 'w' {
                    let mut plus_found = false;
                    let mut asteriks_found = false;
                    let mut question_mark_found = false;

                    if pattern.chars().count() > k + 1 + 1 {
                        //.count() - 1 taken to other side
                        if pattern.chars().nth(k + 2).unwrap() == '+' {
                            plus_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::OneOrMore(Box::new(PatternUnit::AlphaNumeric)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '*' {
                            asteriks_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrMore(Box::new(PatternUnit::AlphaNumeric)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '?' {
                            question_mark_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrOne(Box::new(PatternUnit::AlphaNumeric)),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        }
                    }

                    if !plus_found && !asteriks_found && !question_mark_found {
                        //pattern_parts.push(PatternUnit::AlphaNumeric);
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::AlphaNumeric,
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                    }

                    k += 1;
                    k += 1;
                    continue;
                }

                if ('1'..'9')
                    .collect::<Vec<char>>()
                    .as_slice()
                    .contains(&pattern.chars().nth(k + 1).unwrap())
                {
                    let mut plus_found = false;
                    let mut asteriks_found = false;
                    let mut question_mark_found = false;

                    if pattern.chars().count() > k + 1 + 1 {
                        //.count() - 1 taken to other side
                        if pattern.chars().nth(k + 2).unwrap() == '+' {
                            plus_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::OneOrMore(Box::new(PatternUnit::BackReference(
                                    pattern.chars().nth(k + 1).unwrap().to_digit(10).unwrap()
                                        as u16,
                                ))),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '*' {
                            asteriks_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrMore(Box::new(PatternUnit::BackReference(
                                    pattern.chars().nth(k + 1).unwrap().to_digit(10).unwrap()
                                        as u16,
                                ))),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        } else if pattern.chars().nth(k + 2).unwrap() == '?' {
                            question_mark_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrOne(Box::new(PatternUnit::BackReference(
                                    pattern.chars().nth(k + 1).unwrap().to_digit(10).unwrap()
                                        as u16,
                                ))),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );

                            k += 1;
                        }
                    }

                    if !plus_found && !asteriks_found && !question_mark_found {
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::BackReference(
                                pattern.chars().nth(k + 1).unwrap().to_digit(10).unwrap() as u16,
                            ),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                    }

                    k += 1;
                    k += 1;
                    continue;
                }
            } else if pattern.chars().nth(k).unwrap() == '.' {
                if text_buffer != "" {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    text_buffer = String::new();
                }

                let mut plus_found = false;
                let mut asteriks_found = false;
                let mut question_mark_found = false;

                if pattern.chars().count() > k + 1 {
                    //.count() - 1 taken to other side
                    if pattern.chars().nth(k + 1).unwrap() == '+' {
                        plus_found = true;
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::OneOrMore(Box::new(PatternUnit::Wildcard)),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                        k += 1;
                    } else if pattern.chars().nth(k + 1).unwrap() == '*' {
                        asteriks_found = true;
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::ZeroOrMore(Box::new(PatternUnit::Wildcard)),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );

                        k += 1;
                    } else if pattern.chars().nth(k + 1).unwrap() == '?' {
                        question_mark_found = true;
                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::ZeroOrOne(Box::new(PatternUnit::Wildcard)),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                        //pattern_parts.push(PatternUnit::ZeroOrOne(Box::new(PatternUnit::Wildcard)));
                        k += 1;
                    }
                }

                if !plus_found && !asteriks_found && !question_mark_found {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::Wildcard,
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    //pattern_parts.push(PatternUnit::Wildcard);
                }

                k += 1;

                continue;
            } else if pattern.chars().nth(k).unwrap() == '[' {
                if text_buffer != "" {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                    text_buffer = String::new();
                }

                if pattern.chars().count() > k {
                    let mut is_neg = false;
                    let mut kp = k;
                    let mut text_block: String = String::new();
                    if pattern.chars().nth(k + 1).unwrap() == '^' {
                        is_neg = true;
                        kp += 1;
                    }

                    kp += 1;

                    while kp < pattern.chars().count() {
                        if pattern.chars().nth(kp).unwrap() == ']' {
                            let mut plus_found = false;
                            let mut asterkis_found = false;
                            let mut question_mark_found = false;

                            if pattern.chars().count() > kp + 1 {
                                //.count() - 1 taken to other side
                                if pattern.chars().nth(kp + 1).unwrap() == '+' {
                                    plus_found = true;
                                    if is_neg {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::OneOrMore(Box::new(
                                                PatternUnit::NegCharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );
                                        //pattern_parts.push(PatternUnit::OneOrMore(Box::new(
                                        //    PatternUnit::NegCharGroup(text_block.clone()),
                                        //)));
                                    } else {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::OneOrMore(Box::new(
                                                PatternUnit::CharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );
                                        //pattern_parts.push(PatternUnit::OneOrMore(Box::new(
                                        //    PatternUnit::CharGroup(text_block.clone()),
                                        //)));
                                    }
                                    kp += 1;
                                } else if pattern.chars().nth(kp + 1).unwrap() == '*' {
                                    asterkis_found = true;
                                    if is_neg {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::ZeroOrMore(Box::new(
                                                PatternUnit::NegCharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );
                                    } else {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::ZeroOrMore(Box::new(
                                                PatternUnit::CharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );
                                    }
                                    kp += 1;
                                } else if pattern.chars().nth(kp + 1).unwrap() == '?' {
                                    question_mark_found = true;
                                    if is_neg {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::ZeroOrOne(Box::new(
                                                PatternUnit::NegCharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );

                                        //pattern_parts.push(PatternUnit::ZeroOrOne(Box::new(
                                        //    PatternUnit::NegCharGroup(text_block.clone()),
                                        //)));
                                    } else {
                                        add_pattern_unit(
                                            &mut is_alternation,
                                            PatternUnit::ZeroOrOne(Box::new(
                                                PatternUnit::CharGroup(text_block.clone()),
                                            )),
                                            &mut pattern_parts,
                                            pattern_parts_groups_stack_depth,
                                            pattern_parts_groups_stack_alternation_depth,
                                            &mut pattern_parts_group,
                                            &mut currrent_alternation_info,
                                            &mut is_first_alternation,
                                        );

                                        //pattern_parts.push(PatternUnit::ZeroOrOne(Box::new(
                                        //    PatternUnit::CharGroup(text_block.clone()),
                                        // )));
                                    }
                                    kp += 1;
                                }
                            }

                            if !plus_found && !asterkis_found && !question_mark_found {
                                if is_neg {
                                    add_pattern_unit(
                                        &mut is_alternation,
                                        PatternUnit::NegCharGroup(text_block.clone()),
                                        &mut pattern_parts,
                                        pattern_parts_groups_stack_depth,
                                        pattern_parts_groups_stack_alternation_depth,
                                        &mut pattern_parts_group,
                                        &mut currrent_alternation_info,
                                        &mut is_first_alternation,
                                    );
                                    //pattern_parts
                                    //   .push(PatternUnit::NegCharGroup(text_block.clone()));
                                } else {
                                    add_pattern_unit(
                                        &mut is_alternation,
                                        PatternUnit::CharGroup(text_block.clone()),
                                        &mut pattern_parts,
                                        pattern_parts_groups_stack_depth,
                                        pattern_parts_groups_stack_alternation_depth,
                                        &mut pattern_parts_group,
                                        &mut currrent_alternation_info,
                                        &mut is_first_alternation,
                                    );
                                    //pattern_parts.push(PatternUnit::CharGroup(text_block.clone()));
                                }
                            }

                            k = kp;
                            break;
                        } else {
                            text_block.push(pattern.chars().nth(kp).unwrap());
                            kp += 1;
                        }
                    }
                }
            } else if pattern.chars().nth(k).unwrap() == '|' {
                if text_buffer != "" {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                    text_buffer = String::new();
                }

                if is_alternation {
                    is_alternation = false;

                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::Alternation((
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group1.get_mut().clone(),
                            )),
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group2.get_mut().clone(),
                            )),
                        )),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );

                    currrent_alternation_info.group1.get_mut().clear();
                    currrent_alternation_info.group2.get_mut().clear();
                }

                is_first_alternation = true;
                is_alternation = true;
            } else if pattern.chars().nth(k).unwrap() == '(' {
                if text_buffer != "" {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                    text_buffer = String::new();
                }

                init_backreferences.push(String::new());

                if pattern_parts_groups_stack_depth == -1 {
                    pattern_parts_group = PatternUnit::CapturingGroup(vec![]);
                } else {
                    add_child_groupto_group(
                        &mut pattern_parts_group,
                        pattern_parts_groups_stack_depth,
                        0,
                    );
                }

                pattern_parts_groups_stack_depth += 1;
                //
            } else if pattern.chars().nth(k).unwrap() == ')' {
                if text_buffer != "" {
                    add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::PlainText(text_buffer),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );
                    //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                    text_buffer = String::new();
                }

                if is_alternation {
                    is_alternation = false;

                    find_and_push_to_alternation_group(
                        &mut pattern_parts_group,
                        pattern_parts_groups_stack_depth,
                        0,
                        &PatternUnit::Alternation((
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group1.get_mut().clone(),
                            )),
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group2.get_mut().clone(),
                            )),
                        )),
                        &mut currrent_alternation_info,
                        /*&mut is_alternation,
                        &mut pattern_parts,
                        &mut is_first_alternation,*/
                    );

                    /*add_pattern_unit(
                        &mut is_alternation,
                        PatternUnit::Alternation((
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group1.get_mut().clone(),
                            )),
                            Box::new(PatternUnit::NonCapturingGroup(
                                currrent_alternation_info.group2.get_mut().clone(),
                            )),
                        )),
                        &mut pattern_parts,
                        pattern_parts_groups_stack_depth,
                        pattern_parts_groups_stack_alternation_depth,
                        &mut pattern_parts_group,
                        &mut currrent_alternation_info,
                        &mut is_first_alternation,
                    );*/

                    currrent_alternation_info.group1.get_mut().clear();
                    currrent_alternation_info.group2.get_mut().clear();
                }
                pattern_parts_groups_stack_depth -= 1;

                if pattern_parts_groups_stack_depth == -1 {
                    let mut plus_found = false;
                    let mut asteriks_found = false;
                    let mut question_mark_found = false;

                    if pattern.chars().count() > k + 1 {
                        //.count() - 1 taken to other side
                        if pattern.chars().nth(k + 1).unwrap() == '+' {
                            plus_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::OneOrMore(Box::new(pattern_parts_group.clone())),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts.push(PatternUnit::OneOrMore(Box::new(PatternUnit::Wildcard)));
                            k += 1;
                        } else if pattern.chars().nth(k + 1).unwrap() == '*' {
                            asteriks_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrMore(Box::new(pattern_parts_group.clone())),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts
                            //    .push(PatternUnit::ZeroOrMore(Box::new(PatternUnit::Wildcard)));
                            k += 1;
                        } else if pattern.chars().nth(k + 1).unwrap() == '?' {
                            question_mark_found = true;
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::ZeroOrOne(Box::new(pattern_parts_group.clone())),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts.push(PatternUnit::ZeroOrOne(Box::new(PatternUnit::Wildcard)));
                            k += 1;
                        }
                    }

                    if !plus_found && !asteriks_found && !question_mark_found {
                        add_pattern_unit(
                            &mut is_alternation,
                            pattern_parts_group.clone(),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                    }
                }

                /*if pattern_parts_groups_stack_depth == -1 {
                    let mut big_group: PatternUnit;
                    while pattern_parts_groups_stack.len() > 0 {
                        big_group = PatternUnit::CapturingGroup()
                    }
                }*/
            } else {
                let mut plus_found = false;
                let mut asteriks_found = false;
                let mut question_mark_found = false;

                if pattern.chars().count() > k + 1 {
                    //.count() - 1 taken to other side
                    if pattern.chars().nth(k + 1).unwrap() == '+' {
                        plus_found = true;

                        if text_buffer != "" {
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::PlainText(text_buffer),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                            text_buffer = String::new();
                        }

                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::OneOrMore(Box::new(PatternUnit::PlainText(
                                pattern.chars().nth(k).unwrap().to_string(),
                            ))),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );
                        //pattern_parts.push(PatternUnit::OneOrMore(Box::new(
                        //    PatternUnit::PlainText(pattern.chars().nth(k).unwrap().to_string()),
                        //)));

                        k += 1;
                    } else if pattern.chars().nth(k + 1).unwrap() == '*' {
                        asteriks_found = true;

                        if text_buffer != "" {
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::PlainText(text_buffer),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                            text_buffer = String::new();
                        }

                        pattern_parts.push(PatternUnit::ZeroOrMore(Box::new(
                            PatternUnit::PlainText(pattern.chars().nth(k).unwrap().to_string()),
                        )));

                        k += 1;
                    } else if pattern.chars().nth(k + 1).unwrap() == '?' {
                        question_mark_found = true;

                        if text_buffer != "" {
                            //pattern_parts.push(PatternUnit::PlainText(text_buffer));
                            add_pattern_unit(
                                &mut is_alternation,
                                PatternUnit::PlainText(text_buffer),
                                &mut pattern_parts,
                                pattern_parts_groups_stack_depth,
                                pattern_parts_groups_stack_alternation_depth,
                                &mut pattern_parts_group,
                                &mut currrent_alternation_info,
                                &mut is_first_alternation,
                            );
                            text_buffer = String::new();
                        }

                        add_pattern_unit(
                            &mut is_alternation,
                            PatternUnit::ZeroOrOne(Box::new(PatternUnit::PlainText(
                                pattern.chars().nth(k).unwrap().to_string(),
                            ))),
                            &mut pattern_parts,
                            pattern_parts_groups_stack_depth,
                            pattern_parts_groups_stack_alternation_depth,
                            &mut pattern_parts_group,
                            &mut currrent_alternation_info,
                            &mut is_first_alternation,
                        );

                        //pattern_parts.push(PatternUnit::ZeroOrOne(Box::new(
                        //    PatternUnit::PlainText(pattern.chars().nth(k).unwrap().to_string()),
                        //)));

                        k += 1;
                    }
                }

                if !plus_found && !asteriks_found && !question_mark_found {
                    text_buffer.push(pattern.chars().nth(k).unwrap());
                }
            }

            k += 1;
        }

        if text_buffer != "" {
            add_pattern_unit(
                &mut is_alternation,
                PatternUnit::PlainText(text_buffer),
                &mut pattern_parts,
                pattern_parts_groups_stack_depth,
                pattern_parts_groups_stack_alternation_depth,
                &mut pattern_parts_group,
                &mut currrent_alternation_info,
                &mut is_first_alternation,
            );
            //pattern_parts.push(PatternUnit::PlainText(text_buffer));
        }

        if is_alternation {
            is_alternation = false;

            add_pattern_unit(
                &mut is_alternation,
                PatternUnit::Alternation((
                    Box::new(PatternUnit::NonCapturingGroup(
                        currrent_alternation_info.group1.get_mut().clone(),
                    )),
                    Box::new(PatternUnit::NonCapturingGroup(
                        currrent_alternation_info.group2.get_mut().clone(),
                    )),
                )),
                &mut pattern_parts,
                pattern_parts_groups_stack_depth,
                pattern_parts_groups_stack_alternation_depth,
                &mut pattern_parts_group,
                &mut currrent_alternation_info,
                &mut is_first_alternation,
            );

            currrent_alternation_info.group1.get_mut().clear();
            currrent_alternation_info.group2.get_mut().clear();
        }

        if has_starting_pattern {
            *starting_pattern = pattern_parts.remove(0);
        }

        if has_ending_pattern {
            *ending_pattern = pattern_parts.remove(pattern_parts.len() - 1);
        }

        /////////////////////////////

        let mut is_a_match: bool = true;
        for j in 0..input_line.len() {
            is_a_match = true;

            backreferences = init_backreferences.clone();
            let mut str_cursor: usize = j;
            let mut current_group_index: usize = 0;

            if j == 0 && has_starting_pattern {
                match_unitary_pattern(
                    &mut is_a_match,
                    &mut str_cursor,
                    &starting_pattern,
                    input_line,
                    true,
                    false,
                    &mut backreferences,
                    &mut current_group_index,
                    &mut true,
                    None,
                );

                if !is_a_match {
                    break;
                }
            }

            if j > 0 && has_starting_pattern {
                is_a_match = false;
                break;
            }

            for i in 0..pattern_parts.len() {
                if str_cursor >= input_line.len() {
                    is_a_match = false;
                    break;
                }

                let next_pattern = if i + 1 <= pattern_parts.len() - 1 {
                    Some(pattern_parts[i + 1].clone())
                } else {
                    None
                };

                match &pattern_parts[i] {
                    PatternUnit::Digit => {
                        if !('0'..='9')
                            .collect::<Vec<char>>()
                            .as_slice()
                            .contains(&input_line.chars().nth(str_cursor).unwrap())
                        {
                            is_a_match = false;
                            break;
                        }

                        str_cursor += 1;
                    }

                    PatternUnit::AlphaNumeric => {
                        if !(('0'..='9')
                            .collect::<Vec<char>>()
                            .as_slice()
                            .contains(&input_line.chars().nth(str_cursor).unwrap())
                            || ('a'..='z')
                                .collect::<Vec<char>>()
                                .as_slice()
                                .contains(&input_line.chars().nth(str_cursor).unwrap())
                            || ('A'..='Z')
                                .collect::<Vec<char>>()
                                .as_slice()
                                .contains(&input_line.chars().nth(str_cursor).unwrap())
                            || input_line.chars().nth(str_cursor).unwrap() == '_')
                        {
                            is_a_match = false;

                            break;
                        }
                        str_cursor += 1;
                    }

                    PatternUnit::PlainText(plain_text) => {
                        if str_cursor + plain_text.len() <= input_line.len() {
                            let ptxt = &input_line[str_cursor..str_cursor + plain_text.len()];

                            if !(ptxt == plain_text) {
                                is_a_match = false;
                                break;
                            } else {
                            }

                            str_cursor += plain_text.len();
                        } else {
                            is_a_match = false;
                            break;
                        }
                    }

                    PatternUnit::CharGroup(plain_text) => {
                        if !plain_text
                            .chars()
                            .collect::<Vec<char>>()
                            .contains(&input_line.chars().nth(str_cursor).unwrap())
                        {
                            is_a_match = false;
                            break;
                        }

                        str_cursor += 1;
                    }

                    PatternUnit::NegCharGroup(plain_text) => {
                        if plain_text
                            .chars()
                            .collect::<Vec<char>>()
                            .contains(&input_line.chars().nth(str_cursor).unwrap())
                        {
                            is_a_match = false;
                            break;
                        }

                        str_cursor += 1;
                    }

                    PatternUnit::OneOrMore(pu) => {
                        let mut is_local_match: bool = true;

                        match_unitary_pattern(
                            &mut is_local_match,
                            &mut str_cursor,
                            &pu,
                            &input_line,
                            false,
                            false,
                            &mut backreferences,
                            &mut current_group_index,
                            &mut true,
                            next_pattern.clone(),
                        );

                        if str_cursor <= input_line.len() - 1 {
                            match **pu {
                                PatternUnit::NegCharGroup(ref _ha) => {
                                    if str_cursor + 1 > input_line.len() - 1 {
                                        is_a_match = is_local_match;
                                        break;
                                    }
                                }
                                _ => {}
                            }

                            if !is_local_match {
                                is_a_match = is_local_match;
                                break;
                            }

                            let mut is_match_temp: bool = false;
                            if **pu == PatternUnit::Wildcard {
                                is_match_temp = true;
                                if i + 1 <= pattern_parts.len() - 1 {
                                    let lpu = &pattern_parts[i + 1].clone();
                                    let mut str_cursor_temp = str_cursor.clone();

                                    match_unitary_pattern(
                                        &mut is_match_temp,
                                        &mut str_cursor_temp,
                                        &lpu,
                                        &input_line,
                                        false,
                                        false,
                                        &mut backreferences,
                                        &mut current_group_index,
                                        &mut false,
                                        next_pattern.clone(),
                                    );
                                }
                            }
                            if is_match_temp {
                                break;
                            }

                            while is_local_match {
                                let mut is_match_temp: bool = false;
                                if **pu == PatternUnit::Wildcard {
                                    is_match_temp = true;
                                    if i + 1 <= pattern_parts.len() - 1 {
                                        let lpu = &pattern_parts[i + 1].clone();
                                        let mut str_cursor_temp = str_cursor.clone();

                                        match_unitary_pattern(
                                            &mut is_match_temp,
                                            &mut str_cursor_temp,
                                            &lpu,
                                            &input_line,
                                            false,
                                            false,
                                            &mut backreferences,
                                            &mut current_group_index,
                                            &mut false,
                                            next_pattern.clone(),
                                        );
                                    }
                                }
                                if is_match_temp {
                                    break;
                                }

                                if str_cursor > &input_line.len() - 1 {
                                    break;
                                }

                                match_unitary_pattern(
                                    &mut is_local_match,
                                    &mut str_cursor,
                                    &pu,
                                    &input_line,
                                    false,
                                    false,
                                    &mut backreferences,
                                    &mut current_group_index,
                                    &mut false,
                                    next_pattern.clone(),
                                );

                                if str_cursor > input_line.len() - 1 {
                                    break;
                                }

                                match **pu {
                                    PatternUnit::NegCharGroup(ref _ha) => {
                                        if str_cursor + 1 > input_line.len() - 1 {
                                            is_a_match = is_local_match;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    PatternUnit::ZeroOrMore(pu) => {
                        let mut is_local_match: bool = true;

                        let mut is_match_temp: bool = false;
                        if **pu == PatternUnit::Wildcard {
                            is_match_temp = true;
                            if i + 1 <= pattern_parts.len() - 1 {
                                let lpu = &pattern_parts[i + 1].clone();
                                let mut str_cursor_temp = str_cursor.clone();

                                match_unitary_pattern(
                                    &mut is_match_temp,
                                    &mut str_cursor_temp,
                                    &lpu,
                                    &input_line,
                                    false,
                                    false,
                                    &mut backreferences,
                                    &mut current_group_index,
                                    &mut false,
                                    next_pattern.clone(),
                                );
                            }
                        }
                        if is_match_temp {
                            break; //check if next pattern matches with current cursor point, if yes we break
                        }

                        match_unitary_pattern(
                            &mut is_local_match,
                            &mut str_cursor,
                            &pu,
                            &input_line,
                            false,
                            false,
                            &mut backreferences,
                            &mut current_group_index,
                            &mut true,
                            next_pattern.clone(),
                        );

                        if str_cursor <= input_line.len() - 1 {
                            match **pu {
                                PatternUnit::NegCharGroup(ref _ha) => {
                                    if str_cursor + 1 > input_line.len() - 1 {
                                        is_a_match = is_local_match;
                                        break;
                                    }
                                }
                                _ => {}
                            }

                            while is_local_match {
                                let mut is_match_temp: bool = false;

                                if **pu == PatternUnit::Wildcard {
                                    is_match_temp = true;
                                    if i + 1 <= pattern_parts.len() - 1 {
                                        let lpu = &pattern_parts[i + 1].clone();
                                        let mut str_cursor_temp = str_cursor.clone();

                                        match_unitary_pattern(
                                            &mut is_match_temp,
                                            &mut str_cursor_temp,
                                            &lpu,
                                            &input_line,
                                            false,
                                            false,
                                            &mut backreferences,
                                            &mut current_group_index,
                                            &mut false,
                                            next_pattern.clone(),
                                        );
                                    }
                                }

                                if is_match_temp {
                                    //check if next pattern matches with current cursor point, if yes we break
                                    break;
                                }

                                if str_cursor > &input_line.len() - 1 {
                                    break;
                                }

                                match_unitary_pattern(
                                    &mut is_local_match,
                                    &mut str_cursor,
                                    &pu,
                                    &input_line,
                                    false,
                                    false,
                                    &mut backreferences,
                                    &mut current_group_index,
                                    &mut false,
                                    next_pattern.clone(),
                                );

                                if str_cursor > &input_line.len() - 1 {
                                    break;
                                }

                                match **pu {
                                    PatternUnit::NegCharGroup(ref _ha) => {
                                        if str_cursor + 1 > input_line.len() - 1 {
                                            is_a_match = is_local_match;
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    PatternUnit::ZeroOrOne(pu) => {
                        let mut is_local_match: bool = true;

                        match_unitary_pattern(
                            &mut is_local_match,
                            &mut str_cursor,
                            &pu,
                            &input_line,
                            false,
                            false,
                            &mut backreferences,
                            &mut current_group_index,
                            &mut true,
                            next_pattern.clone(),
                        );
                    }

                    PatternUnit::Wildcard => {
                        str_cursor += 1;
                    }

                    PatternUnit::Alternation((pu1, pu2)) => {
                        let mut is_local_match1: bool = true;
                        let mut is_local_match2: bool = true;

                        let mut str_cursor_cpy1 = str_cursor.clone();
                        let mut str_cursor_cpy2 = str_cursor.clone();

                        match_unitary_pattern(
                            &mut is_local_match1,
                            &mut str_cursor_cpy1,
                            &pu1,
                            &input_line,
                            false,
                            false,
                            &mut backreferences,
                            &mut current_group_index,
                            &mut true,
                            next_pattern.clone(),
                        );

                        match_unitary_pattern(
                            &mut is_local_match2,
                            &mut str_cursor_cpy2,
                            &pu2,
                            &input_line,
                            false,
                            false,
                            &mut backreferences,
                            &mut current_group_index,
                            &mut true,
                            next_pattern.clone(),
                        );

                        if is_local_match1 {
                            str_cursor = str_cursor_cpy1;
                        } else if is_local_match2 {
                            str_cursor = str_cursor_cpy2;
                        }

                        is_a_match = is_local_match1 || is_local_match2;
                    }

                    PatternUnit::CapturingGroup(puvec) => {
                        let mut is_local_match: bool = true;
                        let str_cursor_start = str_cursor;

                        current_group_index += 1;

                        let this_group_index = (current_group_index).clone();

                        //let mut backref_buffer: String = String::new();
                        for k in 0..puvec.len() {
                            let local_next_pattern = if k + 1 <= puvec.len() - 1 {
                                Some(puvec[k + 1].clone())
                            } else {
                                next_pattern.clone()
                            };

                            is_local_match = true;
                            if str_cursor < input_line.len() {
                                //backref_buffer
                                //    .push(input_line.chars().nth(str_cursor).unwrap().clone());
                            } else {
                                is_local_match = false;
                                is_a_match = false;
                                break;
                            }

                            match_unitary_pattern(
                                &mut is_local_match,
                                &mut str_cursor,
                                &puvec[k],
                                &input_line,
                                false,
                                false,
                                &mut backreferences,
                                &mut current_group_index,
                                &mut true,
                                local_next_pattern.clone(),
                            );

                            //

                            if !is_local_match {
                                is_a_match = false;
                                break;
                            }
                        }

                        if is_local_match {
                            //backreferences
                            //    .push(input_line[str_cursor_start..str_cursor].to_string());
                            //
                            backreferences[this_group_index - 1] =
                                input_line[str_cursor_start..str_cursor].to_string();
                        } else {
                        }
                    }

                    PatternUnit::NonCapturingGroup(puvec) => {
                        let mut is_local_match: bool = true;
                        //let mut backref_buffer: String = String::new();
                        for k in 0..puvec.len() {
                            let local_next_pattern = if k + 1 <= puvec.len() - 1 {
                                Some(puvec[k + 1].clone())
                            } else {
                                next_pattern.clone()
                            };

                            is_local_match = true;
                            if str_cursor < input_line.len() {
                                //backref_buffer
                                //    .push(input_line.chars().nth(str_cursor).unwrap().clone());
                            } else {
                                is_local_match = false;
                                break;
                            }

                            match_unitary_pattern(
                                &mut is_local_match,
                                &mut str_cursor,
                                &puvec[k],
                                &input_line,
                                false,
                                false,
                                &mut backreferences,
                                &mut current_group_index,
                                &mut true,
                                local_next_pattern.clone(),
                            );

                            if !is_local_match {
                                is_a_match = false;
                                break;
                            }
                        }
                    }

                    PatternUnit::BackReference(index) => {
                        let mut is_local_match: bool = true;

                        if (*index as usize) - 1 < backreferences.len() {
                            match_unitary_pattern(
                                &mut is_local_match,
                                &mut str_cursor,
                                &PatternUnit::PlainText(
                                    backreferences[(*index - 1) as usize].clone(),
                                ),
                                &input_line,
                                false,
                                false,
                                &mut backreferences,
                                &mut current_group_index,
                                &mut true,
                                next_pattern.clone(),
                            );
                        } else {
                            is_local_match = false;
                        }

                        if !is_local_match {
                            is_a_match = false;
                        }
                    }
                }
            }

            if pattern_parts.len() == 0 && has_ending_pattern {
                str_cursor =
                    input_line.len() - (ending_pattern_last_spacing(&ending_pattern) as usize);
            }

            if has_ending_pattern && is_a_match {
                match_unitary_pattern(
                    &mut is_a_match,
                    &mut str_cursor,
                    &ending_pattern,
                    input_line,
                    false,
                    true,
                    &mut backreferences,
                    &mut current_group_index,
                    &mut true,
                    None,
                );
            }

            if is_a_match {
                break;
            }

            if !is_a_match {}
        }

        return is_a_match;
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_1() {
        assert_eq!(match_pattern("aaaa", r"\d"), false);
    }

    #[test]
    fn digits_2() {
        assert_eq!(match_pattern("111", r"\d"), true);
    }

    #[test]
    fn alpha_num_3() {
        assert_eq!(match_pattern("aaa11", r"\d"), true);
    }

    #[test]
    fn alpha_num_1() {
        assert_eq!(match_pattern("a1", r"\w"), true);
    }

    #[test]
    fn alpha_num_2() {
        assert_eq!(match_pattern("?!?", r"\w"), false);
    }

    #[test]
    fn alpha_num_4() {
        assert_eq!(match_pattern("abc1xyz", r"abc\dxyz"), true);
    }

    #[test]
    fn alpha_num_5() {
        assert_eq!(match_pattern("abchxyz", r"abc\dxyz"), false);
    }

    #[test]
    fn positive_char_group_1() {
        assert_eq!(match_pattern("a", "[bbabb]"), true);
    }

    #[test]
    fn positive_char_group_2() {
        assert_eq!(match_pattern("a", "[bbbb]"), false);
    }

    #[test]
    fn negative_char_group() {
        assert_eq!(match_pattern("aaa", "[^bbc]"), true);
    }

    #[test]
    fn negative_char_group_1() {
        assert_eq!(match_pattern("aaaa", "[^abb]"), false);
    }

    #[test]
    fn combine_character_class() {
        assert_eq!(match_pattern("aaaa", r"\w\w\d azeaz"), false);
    }

    #[test]
    fn combine_character_class_1() {
        assert_eq!(match_pattern("aaaa", r"\w\w\dazeaz"), false);
    }

    #[test]
    fn combine_character_class_2() {
        assert_eq!(match_pattern("aaaa", r"\d\d\wdazeaz\w\d"), false);
    }

    #[test]
    fn combine_character_class_3() {
        assert_eq!(match_pattern("11XaaC2", r"\d\d\waa\w\d"), true);
    }

    #[test]
    fn combine_character_class_4() {
        assert_eq!(match_pattern("Xx6azeaz", r"\w\w\dazeaz"), true);
    }

    #[test]
    fn starting_pattern() {
        assert_eq!(match_pattern("abcooo", r"^abc"), true);
    }

    #[test]
    fn starting_pattern_1() {
        assert_eq!(match_pattern("1abcXx6azeaz", r"^\dabc"), true);
    }

    #[test]
    fn starting_pattern_2() {
        assert_eq!(match_pattern("XabcXx6azeaz", r"^\wabc"), true);
    }
    #[test]
    fn starting_pattern_3() {
        assert_eq!(match_pattern("XabcXx6azeaz", r"^\wxabc"), false);
    }

    #[test]
    fn starting_pattern_4() {
        assert_eq!(match_pattern("1XabcXx6azeaz", r"^\wyabc"), false);
    }

    #[test]
    fn starting_pattern_5() {
        assert_eq!(match_pattern("abcooo", r"^dabc"), false);
    }

    #[test]
    fn ending_pattern() {
        assert_eq!(match_pattern("abc", r"abc$"), true);
    }

    #[test]
    fn ending_pattern_1() {
        assert_eq!(match_pattern("abcooo", r"abc$"), false);
    }

    #[test]
    fn ending_pattern_2() {
        assert_eq!(match_pattern("abcX", r"\w$"), true);
    }

    #[test]
    fn ending_pattern_3() {
        assert_eq!(match_pattern("abcooo?", r"\w$"), false);
    }

    #[test]
    fn ending_pattern_4() {
        assert_eq!(match_pattern("abc1", r"\d$"), true);
    }

    #[test]
    fn ending_pattern_5() {
        assert_eq!(match_pattern("abcooo", r"\d$"), false);
    }

    #[test]
    fn ending_pattern_6() {
        assert_eq!(match_pattern("abcooo", r"[oax]$"), true);
    }

    #[test]
    fn ending_pattern_7() {
        assert_eq!(match_pattern("vvvvvx", r"[oax]$"), true);
    }

    #[test]
    fn ending_pattern_8() {
        assert_eq!(match_pattern("abcozb", r"[oax]$"), false);
    }

    #[test]
    fn one_or_more() {
        assert_eq!(match_pattern("aav3bf", r"aav\d+bf"), true);
    }

    #[test]
    fn one_or_more_1() {
        assert_eq!(match_pattern("aav333bf", r"aav\d+bf"), true);
    }

    #[test]
    fn one_or_more_2() {
        assert_eq!(match_pattern("aavbf", r"aav\d+bf"), false);
    }

    #[test]
    fn one_or_more_3() {
        assert_eq!(match_pattern("aavbf", r"aav+bf"), true);
    }

    #[test]
    fn one_or_more_4() {
        assert_eq!(match_pattern("aavvvvvbf", r"aav+bf"), true);
    }

    #[test]
    fn one_or_more_5() {
        assert_eq!(match_pattern("aabf", r"aav+bf"), false);
    }

    #[test]
    fn zero_or_more() {
        assert_eq!(match_pattern("aav3bf", r"aav\d*bf"), true);
    }

    #[test]
    fn zerp_or_more_1() {
        assert_eq!(match_pattern("aav333bf", r"aav\d*bf"), true);
    }

    #[test]
    fn zerp_or_more_2() {
        assert_eq!(match_pattern("aavbf", r"aav\d*bf"), true);
    }

    #[test]
    fn zerp_or_more_3() {
        assert_eq!(match_pattern("aavbf", r"aav*bf"), true);
    }

    #[test]
    fn zerp_or_more_4() {
        assert_eq!(match_pattern("aavvvvvbf", r"aav*bf"), true);
    }

    #[test]
    fn zerp_or_more_5() {
        assert_eq!(match_pattern("aabf", r"aav*bf"), true);
    }

    #[test]
    fn zerp_or_more_6() {
        assert_eq!(match_pattern("aa2bf", r"aav*bf"), false);
    }

    #[test]
    fn zerp_or_more_7() {
        assert_eq!(match_pattern("aabbf", r"aa\d*bf"), false);
    }

    #[test]
    fn zerp_or_more_8() {
        assert_eq!(match_pattern("c", r"v*"), true);
    }

    #[test]
    fn zerp_or_more_9() {
        assert_eq!(match_pattern("2", r"v*\d"), true);
    }

    #[test]
    fn zerp_or_more_10() {
        assert_eq!(match_pattern("c", r"abv*"), false);
    }

    #[test]
    fn zerp_or_more_11() {
        assert_eq!(match_pattern("?????", r"abv"), false);
    }

    #[test]
    fn zerp_or_one() {
        assert_eq!(match_pattern("a", r"\d?"), true);
    }

    #[test]
    fn zerp_or_one_1() {
        assert_eq!(match_pattern("abcv", r"ab\d?v"), false);
    }

    #[test]
    fn wildcard() {
        assert_eq!(match_pattern("abc", r"."), true);
    }

    #[test]
    fn wildcard_1() {
        assert_eq!(match_pattern("abc", r"a.c"), true);
    }

    #[test]
    fn wildcard_2() {
        assert_eq!(match_pattern("abc", r"a.b"), false);
    }

    #[test]
    fn wildcard_3() {
        assert_eq!(match_pattern("azerverbc", r"a.+c"), true);
    }

    #[test]
    fn wildcard_4() {
        assert_eq!(match_pattern("azerverb2", r"a.+c"), false);
    }

    #[test]
    fn alternation() {
        assert_eq!(match_pattern("abc", r"abc|xyz"), true);
    }

    #[test]
    fn alternation_1() {
        assert_eq!(match_pattern("xyz", r"abc|xyz"), true);
    }

    #[test]
    fn alternation_2() {
        assert_eq!(match_pattern("jhk", r"abc|xyz"), false);
    }
    #[test]
    fn alternation_3() {
        assert_eq!(match_pattern("ab", r"abc|xyz\w"), false);
    }

    #[test]
    fn alternation_4() {
        assert_eq!(match_pattern("2", r"abc|\d"), true);
    }

    #[test]
    fn alternation_5() {
        assert_eq!(match_pattern("a", r"a (cat|dog)"), false);
    }

    #[test]
    fn alternation_6() {
        assert_eq!(match_pattern("cat", r"a (cat|dog)"), false);
    }

    #[test]
    fn alternation_7() {
        assert_eq!(match_pattern("a cat", r"a (cat|dog)"), true);
    }

    #[test]
    fn alternation_8() {
        assert_eq!(match_pattern("a dog", r"a (cat|dog)"), true);
    }
    #[test]
    fn alternation_10() {
        assert_eq!(match_pattern("dogs", r"(cat|dog)s"), true);
    }
    #[test]
    fn alternation_11() {
        assert_eq!(match_pattern("kittys", r"(kitty|dog)s"), true);
    }

    #[test]
    fn alternation_9() {
        assert_eq!(match_pattern("dog", r"a (cat|dog)"), false);
    }

    #[test]
    fn backreference() {
        assert_eq!(match_pattern("dog and dog", r"(dog) and \1"), true);
    }

    #[test]
    fn backreference_1() {
        assert_eq!(match_pattern("dog and cat", r"(dog) and \1"), false);
    }

    #[test]
    fn backreference_2() {
        assert_eq!(
            match_pattern(
                "$?! 101 is doing $?! 101 times",
                r"(\w\w\w \d\d\d) is doing \1 times"
            ),
            false
        );
    }

    #[test]
    fn backreference_3() {
        assert_eq!(
            match_pattern(
                "ZZZ 101 is doing ZZZ 101 times",
                r"(\w\w\w \d\d\d) is doing \1 times"
            ),
            true
        );
    }

    #[test]
    fn backreference_4() {
        assert_eq!(
            match_pattern("abcd is abcd, not efg", r"([abcd]+) is \1, not [^xyz]+"),
            true
        );
    }

    #[test]
    fn backreference_5() {
        assert_eq!(
            match_pattern(
                "abcd is abcd, and xyzz is xyzz",
                r"([abcd]+) is \1, and ([xyz]+) is \2"
            ),
            true
        );
    }

    #[test]
    fn backreference_6() {
        assert_eq!(
            match_pattern(
                "this starts and ends with this",
                r"^(\w+) starts and ends with \1$"
            ),
            true
        );
    }

    #[test]
    fn backreference_7() {
        assert_eq!(
            match_pattern(
                "this starts and ends with this?",
                r"^(\w+) starts and ends with \1$"
            ),
            false
        );
    }

    #[test]
    fn backreference_8() {
        assert_eq!(
            match_pattern("bugs here and bugs there", r"(b..s|c..e) here and \1 there"),
            true
        );
    }

    #[test]
    fn backreference_9() {
        assert_eq!(
            match_pattern(
                "3 red squares and 4 red circles",
                r"(\d+) (\w+) squares and \1 \2 circles"
            ),
            false
        );
    }

    #[test]
    fn backreference_10() {
        assert_eq!(
            match_pattern(
                "cat and cat' is the same as 'cat and cat'",
                r"('(cat) and \2') is the same as \1"
            ),
            false
        );
    }

    #[test]
    fn backreference_11() {
        assert_eq!(
            match_pattern(
                "'cat and cat' is the same as 'cat and dog'",
                r"('(cat) and \2') is the same as \1"
            ),
            false
        );
    }

    #[test]
    fn backreference_12() {
        assert_eq!(
            match_pattern(
                "$?! 101 is doing $?! 101 times, and again $?! 101 times",
                r"((\w\w\w) (\d\d\d)) is doing \2 \3 times, and again \1 times"
            ),
            false
        );
    }

    #[test]
    fn backreference_13() {
        assert_eq!(
            match_pattern(
                "abc-def is abc-def, not efg, abc, or def",
                r"(([abc]+)-([def]+)) is \1, not ([^xyz]+), \2, or \3"
            ),
            true
        );
    }

    #[test]
    fn backreference_14() {
        assert_eq!(
            match_pattern(
                "apple pie is made of apple and pie. love apple pie",
                r"^((\w+) (\w+)) is made of \2 and \3. love \1$"
            ),
            true
        );
    }

    #[test]
    fn backreference_15() {
        assert_eq!(
            match_pattern(
                "cat and fish, cat with fish, cat and fish",
                r"((c.t|d.g) and (f..h|b..d)), \2 with \3, \1"
            ),
            true
        );
    }

    #[test]
    fn backreference_16() {
        assert_eq!(
            match_pattern(
                "lui and fish, lui with fish, lui and fish",
                r"((c.t|d.g|l.l) and (f..h|b..d)), \2 with \3, \1"
            ),
            false
        );
    }

    #[test]
    fn backreference_17() {
        assert_eq!(
            match_pattern(
                "lol and fish, lol with fish, lol and fish",
                r"((c.t|d.g|l.l) and (f..h|b..d)), \2 with \3, \1"
            ),
            true
        );
    }

    #[test]
    fn groups() {
        assert_eq!(match_pattern("abcafg", r"(abc)(afg)"), true);
    }

    #[test]
    fn groups_1() {
        assert_eq!(match_pattern("abcWafg", r"(abc\w)(afg)"), true);
    }
    #[test]
    fn groups_2() {
        assert_eq!(match_pattern("abcKafgafgafg", r"(abc\w)(afg)+"), true);
    }

    #[test]
    fn groups_3() {
        assert_eq!(match_pattern("abcjkiabcjki", r"(abc(jki))+"), true);
    }

    #[test]
    fn groups_4() {
        assert_eq!(match_pattern("abcjkihihi", r"(abc(jki(hihi)))"), true);
    }

    #[test]
    fn groups_5() {
        assert_eq!(match_pattern("abcjkihihi5", r"(abc(jki(hihi)))+\d"), true);
    }
}
