use super::*;

#[test]
fn normal_can_enter_visual_mode_and_escape_back() {
    harness("first\nsecond")
        .keys("v")
        .assert_mode(Mode::Visual)
        .keys("<Esc>")
        .assert_mode(Mode::Normal);
}

#[test]
fn normal_can_enter_command_mode_and_escape_back() {
    harness("first")
        .keys(":")
        .assert_mode(Mode::Command)
        .keys("<Esc>")
        .assert_mode(Mode::Normal);
}

#[test]
fn insertion_enters_insert_mode_and_escape_cancels_back_to_normal() {
    harness("first")
        .keys("i")
        .assert_mode(Mode::Insert)
        .keys("<Esc>")
        .assert_mode(Mode::Normal)
        .assert_annotation_count(0);
}

#[test]
fn visual_deletion_returns_to_normal_mode() {
    harness("first\nsecond")
        .keys("vld")
        .assert_mode(Mode::Normal)
        .assert_annotation_count(1);
}

#[test]
fn normal_movement_keys_update_cursor_position() {
    harness("abcd\nefgh")
        .keys("ljh")
        .assert_cursor(1, 0)
        .keys("k")
        .assert_cursor(0, 0);
}

#[test]
fn gg_moves_to_top_of_document() {
    harness("one\ntwo\nthree").keys("jjgg").assert_cursor(0, 0);
}

#[test]
fn shift_g_moves_to_bottom_of_document() {
    harness("one\ntwo\nthree").keys("G").assert_cursor(2, 0);
}

#[test]
fn zero_and_dollar_move_to_line_start_and_end() {
    harness("abcd\nefgh")
        .keys("jl0")
        .assert_cursor(1, 0)
        .keys("$")
        .assert_cursor(1, 3);
}
