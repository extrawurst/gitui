// bit for modifiers
// bits: 0  None 
// bits: 1  SHIFT
// bits: 2  CONTROL
//
// Note:
// If the default key layout is lower case,
// and you want to use `Shift + q` to trigger the exit event,
// the setting should like this `exit: Some(( code: Char('Q'), modifiers: ( bits: 1,),)),`
// The Char should be upper case, and the shift modified bit should be set to 1.
//
// Note:
// find `KeysList` type in src/keys/key_list.rs for all possible keys.
// every key not overwritten via the config file will use the default specified there
(
    open_help: Some(( code: F(1), modifiers: ( bits: 0,),)),

    move_left: Some(( code: Char('h'), modifiers: ( bits: 0,),)),
    move_right: Some(( code: Char('l'), modifiers: ( bits: 0,),)),
    move_up: Some(( code: Char('k'), modifiers: ( bits: 0,),)),
    move_down: Some(( code: Char('j'), modifiers: ( bits: 0,),)),
    
    popup_up: Some(( code: Char('p'), modifiers: ( bits: 2,),)),
    popup_down: Some(( code: Char('n'), modifiers: ( bits: 2,),)),
    page_up: Some(( code: Char('b'), modifiers: ( bits: 2,),)),
    page_down: Some(( code: Char('f'), modifiers: ( bits: 2,),)),
    home: Some(( code: Char('g'), modifiers: ( bits: 0,),)),
    end: Some(( code: Char('G'), modifiers: ( bits: 1,),)),
    shift_up: Some(( code: Char('K'), modifiers: ( bits: 1,),)),
    shift_down: Some(( code: Char('J'), modifiers: ( bits: 1,),)),

    edit_file: Some(( code: Char('I'), modifiers: ( bits: 1,),)),

    status_reset_item: Some(( code: Char('U'), modifiers: ( bits: 1,),)),

    diff_reset_lines: Some(( code: Char('u'), modifiers: ( bits: 0,),)),
    diff_stage_lines: Some(( code: Char('s'), modifiers: ( bits: 0,),)),

    stashing_save: Some(( code: Char('w'), modifiers: ( bits: 0,),)),
    stashing_toggle_index: Some(( code: Char('m'), modifiers: ( bits: 0,),)),

    stash_open: Some(( code: Char('l'), modifiers: ( bits: 0,),)),

    abort_merge: Some(( code: Char('M'), modifiers: ( bits: 1,),)),
)
