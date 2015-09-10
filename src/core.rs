extern crate alloc;
extern crate test;

use self::alloc::{heap, oom};
use std::mem;
use std::ptr::{Unique, self};
use std::str::FromStr;
use std::ascii::AsciiExt;
use std::fmt;
use std::slice;
use std::io::Write;
use byteorder::{ByteOrder, NativeEndian, WriteBytesExt};


use exception::Exception::{
    self,
    NoException,
    Abort,
    UnexpectedEndOfFile,
    UndefinedWord,
    StackOverflow,
    StackUnderflow,
    ReturnStackUnderflow,
    ReturnStackOverflow,
    UnsupportedOperation,
    InterpretingACompileOnlyWord,
    Bye,
};

// Word
pub struct Word {
    is_immediate: bool,
    is_compile_only: bool,
    hidden: bool,
    nfa: usize,
    dfa: usize,
    name_len: usize,
    action: fn(& mut VM) -> Option<Exception>
}

impl Word {
    pub fn new(nfa: usize, name_len: usize, dfa: usize, action: fn(& mut VM) -> Option<Exception>) -> Word {
        Word {
            is_immediate: false,
            is_compile_only: false,
            hidden: false,
            nfa: nfa,
            dfa: dfa,
            name_len: name_len,
            action: action
        }
    }

    pub fn nfa(&self) -> usize {
        self.nfa
    }

    pub fn dfa(&self) -> usize {
        self.dfa
    }

    pub fn name_len(&self) -> usize {
        self.name_len
    }

}

pub trait Heap {
    fn push_f64(&mut self, v: f64);
    fn get_f64(&self, pos: usize) -> f64;
    fn put_f64(&mut self, pos: usize, v: f64);
    fn push_i32(&mut self, v: i32);
    fn get_i32(&self, pos: usize) -> i32;
    fn put_i32(&mut self, pos: usize, v: i32);
    fn get_u8(&self, pos: usize) -> u8;
    fn put_u8(&mut self, pos: usize, v: u8);
}

impl Heap for Vec<u8> {
    fn push_f64(&mut self, v: f64) {
        self.write_f64::<NativeEndian>(v).unwrap();
    }
    fn get_f64(&self, pos: usize) -> f64 {
        NativeEndian::read_f64(&self[pos..])
    }
    fn put_f64(&mut self, pos: usize, v: f64) {
        NativeEndian::write_f64(&mut self[pos..], v);
    }
    fn push_i32(&mut self, v: i32) {
        self.write_i32::<NativeEndian>(v).unwrap();
    }
    fn get_i32(&self, pos: usize) -> i32 {
        NativeEndian::read_i32(&self[pos..])
    }
    fn put_i32(&mut self, pos: usize, v: i32) {
        NativeEndian::write_i32(&mut self[pos..], v);
    }
    fn get_u8(&self, pos: usize) -> u8 {
        self[pos]
    }
    fn put_u8(&mut self, pos: usize, v: u8) {
        self[pos] = v;
    }
}

pub struct Stack<T> {
    ptr: Unique<T>,
    cap: usize,
    len: usize
}

impl<T> Stack<T> {
    fn with_capacity(cap: usize) -> Self {
        assert!(cap > 0 && cap <= 2048, "Invalid stack capacity");
        let align = mem::align_of::<isize>();
        let elem_size = mem::size_of::<isize>();
        unsafe {
            let ptr = heap::allocate(cap*elem_size, align);
            if ptr.is_null() { oom(); }
            Stack{ ptr: Unique::new(ptr as *mut _), cap: cap, len: 0 }
        }
    }

    pub fn push(&mut self, v: T) -> Option<T> {
        if self.len >= self.cap {
            Some(v)
        } else {
            unsafe {
                ptr::write(self.ptr.offset(self.len as isize), v);
            }
            self.len += 1;
            None
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len < 1 {
            None
        } else {
            self.len -= 1;
            unsafe {
                Some(ptr::read(self.ptr.offset(self.len as isize)))
            }
        }
    }

    pub fn push2(&mut self, v1: T, v2: T) -> Option<(T,T)> {
        if self.len + 2 > self.cap {
            Some((v1, v2))
        } else {
            unsafe {
                ptr::write(self.ptr.offset(self.len as isize), v1);
                ptr::write(self.ptr.offset((self.len+1) as isize), v2);
            }
            self.len += 2;
            None
        }
    }

    pub fn push3(&mut self, v1: T, v2: T, v3: T) -> Option<(T,T, T)> {
        if self.len + 3 > self.cap {
            Some((v1, v2, v3))
        } else {
            unsafe {
                ptr::write(self.ptr.offset(self.len as isize), v1);
                ptr::write(self.ptr.offset((self.len+1) as isize), v2);
                ptr::write(self.ptr.offset((self.len+2) as isize), v3);
            }
            self.len += 3;
            None
        }
    }

    pub fn pop2(&mut self) -> Option<(T,T)> {
        if self.len < 2 {
            None
        } else {
            self.len -= 2;
            unsafe {
                Some((
                    ptr::read(self.ptr.offset(self.len as isize)),
                    ptr::read(self.ptr.offset((self.len+1) as isize))
                ))
            }
        }
    }

    pub fn pop3(&mut self) -> Option<(T,T,T)> {
        if self.len < 3 {
            None
        } else {
            self.len -= 3;
            unsafe {
                Some((
                    ptr::read(self.ptr.offset(self.len as isize)),
                    ptr::read(self.ptr.offset((self.len+1) as isize)),
                    ptr::read(self.ptr.offset((self.len+2) as isize)),
                ))
            }
        }
    }

    pub fn last(&self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                Some(ptr::read(self.ptr.offset((self.len-1) as isize)))
            }
        }
    }

    pub fn get(&self, pos: usize) -> Option<T> {
        if pos >= self.len {
            None
        } else {
            unsafe {
                Some(ptr::read(self.ptr.offset(pos as isize)))
            }
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// # Safety
    /// Because the implementer (me) is still learning Rust, it is uncertain if as_slice is safe. 
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.get(), self.len) }
    }
}

impl<T: fmt::Display> fmt::Debug for Stack<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match write!(f, "<{}> ", self.len()) {
            Ok(_) => {
                for i in 0..(self.len()-1) {
                    let v = unsafe { ptr::read(self.ptr.offset(i as isize)) };
                    match write!(f, "{} ", v) {
                        Ok(_) => {},
                        Err(e) => { return Err(e); }
                    }
                }
                Ok(())
            },
            Err(e) => Err(e)
        }
    }
}

// Virtual machine
pub struct VM {
    is_compiling: bool,
    is_paused: bool,
    pub s_stack: Stack<isize>,
    r_stack: Stack<isize>,
    pub f_stack: Stack<f64>,
    pub s_heap: Vec<u8>,
    pub n_heap: String,
    pub word_list: Vec<Word>,
    pub instruction_pointer: usize,
    word_pointer: usize,
    pub idx_lit: usize,
    idx_exit: usize,
    pub idx_flit: usize,
    idx_zero_branch: usize,
    idx_branch: usize,
    idx_do: usize,
    idx_loop: usize,
    idx_plus_loop: usize,
    pub idx_type: usize,
    pub input_buffer: String,
    pub source_index: usize,
    pub last_token: String,
    last_definition: usize,
    pub output_buffer: String,
    pub auto_flush: bool,
}

impl VM {
    pub fn new(heap_size: usize) -> VM {
        let mut vm = VM {
            is_compiling: false,
            is_paused: true,
            s_stack: Stack::with_capacity(64),
            r_stack: Stack::with_capacity(64),
            f_stack: Stack::with_capacity(16),
            s_heap: Vec::with_capacity(heap_size),
            n_heap: String::with_capacity(64),
            word_list: Vec::with_capacity(16),
            instruction_pointer: 0,
            word_pointer: 0,
            idx_lit: 0,
            idx_exit: 0,
            idx_flit: 0,
            idx_zero_branch: 0,
            idx_branch: 0,
            idx_do: 0,
            idx_loop: 0,
            idx_plus_loop: 0,
            idx_type: 0,
            input_buffer: String::with_capacity(128),
            source_index: 0,
            last_token: String::with_capacity(64),
            last_definition: 0,
            output_buffer: String::with_capacity(128),
            auto_flush: true
        };
        // Bytecodes
        vm.add_primitive("noop", VM::noop); // j1, Ngaro, jx
        vm.add_primitive("execute", VM::execute); // jx, eForth
        vm.add_primitive("dup", VM::dup); // j1, Ngaro, jx, eForth
        vm.add_primitive("drop", VM::p_drop); // j1, Ngaro, jx, eForth
        vm.add_primitive("swap", VM::swap); // j1, Ngaro, jx, eForth
        vm.add_primitive("over", VM::over); // j1, jx, eForth
        vm.add_primitive("nip", VM::nip); // j1, jx
        vm.add_primitive("depth", VM::depth); // j1, jx
        vm.add_primitive("0<", VM::zero_less); // eForth
        vm.add_primitive("=", VM::equals); // j1, jx
        vm.add_primitive("<", VM::less_than); // j1, jx
        vm.add_primitive("invert", VM::invert); // j1, jx
        vm.add_primitive("and", VM::and); // j1, Ngaro, jx, eForth
        vm.add_primitive("or", VM::or); // j1, Ngaro, jx, eForth
        vm.add_primitive("xor", VM::xor); // j1, Ngaro, jx, eForth
        vm.add_primitive("lshift", VM::lshift); // jx, Ngaro
        vm.add_primitive("rshift", VM::rshift); // jx
        vm.add_primitive("arshift", VM::arshift); // jx, Ngaro
        vm.add_primitive("1+", VM::one_plus); // Ngaro
        vm.add_primitive("1-", VM::one_minus); // Ngaro, jx
        vm.add_primitive("-", VM::minus); // Ngaro
        vm.add_primitive("+", VM::plus); // j1, Ngaro, jx
        vm.add_primitive("*", VM::star); // Ngaro
        vm.add_primitive("/mod", VM::slash_mod); // Ngaro
        vm.add_primitive("cell+", VM::cell_plus); // eForth
        vm.add_primitive("cells", VM::cells); // eForth
        vm.add_primitive("@", VM::fetch); // j1, jx, eForth
        vm.add_primitive("!", VM::store); // j1, jx, eForth
        vm.add_primitive("char+", VM::char_plus); // eForth
        vm.add_primitive("chars", VM::chars); // eForth
        vm.add_primitive("here", VM::here);
        vm.add_primitive("allot", VM::allot);
        vm.add_primitive("c@", VM::c_fetch);
        vm.add_primitive("c!", VM::c_store);

        // Compile-only bytecodes
        vm.add_compile_only("exit", VM::exit); // j1, jx, eForth
        vm.add_compile_only("lit", VM::lit); // Ngaro, jx, eForth
        vm.add_compile_only("branch", VM::branch); // j1, eForth
        vm.add_compile_only("0branch", VM::zero_branch); // j1, eForth
        vm.add_compile_only(">r", VM::p_to_r); // j1, Ngaro, jx, eForth
        vm.add_compile_only("r>", VM::r_from); // j1, Ngaro, jx, eForth
        vm.add_compile_only("r@", VM::r_fetch); // j1, jx, eForth
        vm.add_compile_only("2>r", VM::two_to_r); // jx
        vm.add_compile_only("2r>", VM::two_r_from); // jx
        vm.add_compile_only("2r@", VM::two_r_fetch); // jx
        vm.add_compile_only("_do", VM::_do); // jx
        vm.add_compile_only("_loop", VM::p_loop); // jx
        vm.add_compile_only("_+loop", VM::p_plus_loop); // jx
        vm.add_compile_only("unloop", VM::unloop); // jx
        vm.add_compile_only("leave", VM::leave); // jx
        vm.add_compile_only("i", VM::p_i); // jx
        vm.add_compile_only("j", VM::p_j); // jx

        // Candidates for bytecodes
        // Ngaro: LOOP, JUMP, RETURN, IN, OUT, WAIT
        // j1: U<, RET, IO@, IO!
        // eForth: UM+, !IO, ?RX, TX!
        // jx: PICK, U<, UM*, UM/MOD, D+, TX, RX, CATCH, THROW, QUOTE, UP!, UP+, PAUSE,

        // Immediate words
        vm.add_immediate("(", VM::imm_paren);
        vm.add_immediate("\\", VM::imm_backslash);
        vm.add_immediate("[", VM::interpret);
        vm.add_immediate_and_compile_only("[char]", VM::bracket_char);
        vm.add_immediate_and_compile_only(";", VM::semicolon);
        vm.add_immediate_and_compile_only("if", VM::imm_if);
        vm.add_immediate_and_compile_only("else", VM::imm_else);
        vm.add_immediate_and_compile_only("then", VM::imm_then);
        vm.add_immediate_and_compile_only("begin", VM::imm_begin);
        vm.add_immediate_and_compile_only("while", VM::imm_while);
        vm.add_immediate_and_compile_only("repeat", VM::imm_repeat);
        vm.add_immediate_and_compile_only("again", VM::imm_again);
        vm.add_immediate_and_compile_only("recurse", VM::imm_recurse);
        vm.add_immediate_and_compile_only("do", VM::imm_do);
        vm.add_immediate_and_compile_only("loop", VM::imm_loop);
        vm.add_immediate_and_compile_only("+loop", VM::imm_plus_loop);

        // Compile-only words

        // More Primitives
        vm.add_primitive("true", VM::p_true);
        vm.add_primitive("false", VM::p_false);
        vm.add_primitive("not", VM::zero_equals);
        vm.add_primitive("0=", VM::zero_equals);
        vm.add_primitive("0>", VM::zero_greater);
        vm.add_primitive("0<>", VM::zero_not_equals);
        vm.add_primitive(">", VM::greater_than);
        vm.add_primitive("<>", VM::not_equals);
        vm.add_primitive("rot", VM::rot);
        vm.add_primitive("2dup", VM::two_dup);
        vm.add_primitive("2drop", VM::two_drop);
        vm.add_primitive("2swap", VM::two_swap);
        vm.add_primitive("2over", VM::two_over);
        vm.add_primitive("pause", VM::pause);
        vm.add_primitive("/", VM::slash);
        vm.add_primitive("mod", VM::p_mod);
        vm.add_primitive("abs", VM::abs);
        vm.add_primitive("negate", VM::negate);
        vm.add_primitive("between", VM::between);
        vm.add_primitive("parse-word", VM::parse_word);;
        vm.add_primitive("char", VM::char);
        vm.add_primitive("parse", VM::parse);
        vm.add_primitive("evaluate", VM::evaluate);;
        vm.add_primitive(":", VM::colon);
        vm.add_primitive("constant", VM::constant);
        vm.add_primitive("variable", VM::variable);
        vm.add_primitive("create", VM::create);
        vm.add_primitive("'", VM::tick);
        vm.add_primitive("]", VM::compile);
        vm.add_primitive(",", VM::comma);
        vm.add_primitive("marker", VM::marker);
        vm.add_primitive("quit", VM::quit);
        vm.add_primitive("abort", VM::abort);
        vm.add_primitive("bye", VM::bye);

        vm.idx_lit = vm.find("lit").expect("lit undefined");
        vm.idx_exit = vm.find("exit").expect("exit undefined");
        vm.idx_zero_branch = vm.find("0branch").expect("0branch undefined");
        vm.idx_branch = vm.find("branch").expect("branch undefined");
        vm.idx_do = vm.find("_do").expect("_do undefined");
        vm.idx_loop = vm.find("_loop").expect("_loop undefined");
        vm.idx_plus_loop = vm.find("_+loop").expect("_+loop undefined");
        // S_heap is beginning with noop, because s_heap[0] should not be used.
        vm.compile_word(0); // NOP
        vm
    }

    pub fn word_pointer(&self) -> usize {
        self.word_pointer
    }

    pub fn add_primitive(&mut self, name: &str, action: fn(& mut VM) -> Option<Exception>) {
        self.word_list.push (Word::new(self.n_heap.len(), name.len(), self.s_heap.len(), action));
        self.n_heap.push_str(name);
    }

    pub fn add_immediate(&mut self, name: &str, action: fn(& mut VM) -> Option<Exception>) {
        self.add_primitive (name, action);
        let word = self.word_list.last_mut().unwrap();
        word.is_immediate = true;
    }

    pub fn add_compile_only(&mut self, name: &str, action: fn(& mut VM) -> Option<Exception>) {
        self.add_primitive (name, action);
        let word = self.word_list.last_mut().unwrap();
        word.is_compile_only = true;
    }

    pub fn add_immediate_and_compile_only(&mut self, name: &str, action: fn(& mut VM) -> Option<Exception>) {
        self.add_primitive (name, action);
        let word = self.word_list.last_mut().unwrap();
        word.is_immediate = true;
        word.is_compile_only = true;
    }

    pub fn execute_word(&mut self, i: usize) -> Option<Exception> {
        self.word_pointer = i;
        (self.word_list[i].action)(self)
    }

    /// Find the word with name 'name'.
    /// If not found returns zero.
    pub fn find(&self, name: &str) -> Option<usize> {
        let mut i = 0usize;
        for w in &self.word_list {
            let n = &self.n_heap[w.nfa .. w.nfa+w.name_len];
            if !w.hidden && n.eq_ignore_ascii_case(name) {
                return Some(i);
            } else {
                i += 1;
            }
        }
        None
    }

// Inner interpreter

    pub fn inner_interpret(&mut self, ip: usize) {
        self.instruction_pointer = ip;
        self.inner();
    }

    #[no_mangle]
    #[inline(never)]
    pub fn inner(&mut self) -> Option<Exception> {
        while self.instruction_pointer > 0 && self.instruction_pointer < self.s_heap.len() {
            let w = self.s_heap.get_i32(self.instruction_pointer) as usize;
            self.instruction_pointer += mem::size_of::<i32>();
            match self.execute_word (w) {
                Some(e) => return Some(e),
                None => {}
            }
        }
        self.instruction_pointer = 0;
        None
    }

// Compiler

    pub fn compile_word(&mut self, word_index: usize) {
        self.s_heap.push_i32(word_index as i32);
    }

    /// Compile integer 'i'.
    fn compile_integer (&mut self, i: isize) {
        self.s_heap.push_i32(self.idx_lit as i32);
        self.s_heap.push_i32(i as i32);
    }

    /// Compile float 'f'.
    fn compile_float (&mut self, f: f64) {
        self.s_heap.push_i32(self.idx_flit as i32);
        self.s_heap.push_f64(f);
    }

// Evaluation

    pub fn interpret(& mut self) -> Option<Exception> {
        self.is_compiling = false;
        None
    }

    pub fn compile(& mut self) -> Option<Exception> {
        self.is_compiling = true;
        None
    }

    pub fn set_source(&mut self, s: &str) {
        self.input_buffer.clear();
        self.input_buffer.push_str(s);
        self.source_index = 0;
    }

    /// Run-time: ( "ccc" -- )
    ///
    /// Parse word delimited by white space, skipping leading white spaces.
    pub fn parse_word(&mut self) -> Option<Exception> {
        self.last_token.clear();
        let source = &self.input_buffer[self.source_index..self.input_buffer.len()];
        let mut cnt = 0;
        for ch in source.chars() {
            cnt = cnt + 1;
            match ch {
                '\t' | '\n' | '\r' | ' ' => {
                    if !self.last_token.is_empty() {
                        break;
                    }
                },
                _ => self.last_token.push(ch)
            };
        }
        self.source_index = self.source_index + cnt;
        None
    }

    /// Run-time: ( "&lt;spaces&gt;name" -- char)
    ///
    /// Skip leading space delimiters. Parse name delimited by a space. Put the value of its first character onto the stack.
    pub fn char(&mut self) -> Option<Exception> {
        self.parse_word();
        match self.last_token.chars().nth(0) {
            Some(c) =>
                match self.s_stack.push(c as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(UnexpectedEndOfFile)
        }
    }

    /// Compilation: ( "&lt;spaces&gt;name" -- )
    ///
    /// Skip leading space delimiters. Parse name delimited by a space. Append the run-time semantics given below to the current definition.
    ///
    /// Run-time: ( -- char )
    ///
    /// Place char, the value of the first character of name, on the stack.
    pub fn bracket_char(&mut self) -> Option<Exception> {
        self.char();
        match self.s_stack.pop() {
            Some(ch) => {
                self.compile_integer(ch);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( char "ccc&lt;char&gt;" -- )
    ///
    /// Parse ccc delimited by the delimiter char.
    pub fn parse(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                self.last_token.clear();
                let source = &self.input_buffer[self.source_index..self.input_buffer.len()];
                let mut cnt = 0;
                for ch in source.chars() {
                    cnt = cnt + 1;
                    if ch as isize == v {
                        break;
                    } else {
                        self.last_token.push(ch);
                    }
                }
                self.source_index = self.source_index + cnt;
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn imm_paren(&mut self) -> Option<Exception> {
        match self.s_stack.push(')' as isize) {
            Some(_) => Some(StackOverflow),
            None => self.parse()
        }
    }

    pub fn imm_backslash(&mut self) -> Option<Exception> {
        self.source_index = self.input_buffer.len(); 
        None
    }

    pub fn evaluate(&mut self) -> Option<Exception> {
        let saved_ip = self.instruction_pointer;
        self.instruction_pointer = 0;
        let mut err = NoException;
        loop {
            self.parse_word();
            if self.last_token.is_empty() {
                self.instruction_pointer = saved_ip;
                return None
            }
            match self.find(&self.last_token) {
                Some(found_index) => {
                    let is_immediate_word;
                    let is_compile_only_word;
                    {
                        let word = &self.word_list[found_index];
                        is_immediate_word = word.is_immediate;
                        is_compile_only_word = word.is_compile_only;
                    }
                    if self.is_compiling && !is_immediate_word {
                        self.compile_word(found_index);
                    } else if !self.is_compiling && is_compile_only_word {
                        err = InterpretingACompileOnlyWord;
                    } else {
                        match self.execute_word(found_index) {
                            Some(e) => err = e,
                            None => {
                                if self.instruction_pointer != 0 {
                                    self.inner();
                                }
                            }
                        };
                    }
                },
                None =>
                    // Integer?
                    match FromStr::from_str(&self.last_token) {
                        Ok(t) => {
                            if self.is_compiling {
                                self.compile_integer(t);
                            } else {
                                self.s_stack.push (t);
                            }
                            continue
                        },
                        Err(_) => {
                            // Floating point?
                            match FromStr::from_str(&self.last_token) {
                                Ok(t) => {
                                    if self.idx_flit == 0 {
                                        print!("{} ", "Floating point");
                                        err = UnsupportedOperation;
                                    } else {
                                        if self.is_compiling {
                                            self.compile_float(t);
                                        } else {
                                            self.f_stack.push (t);
                                        }
                                        continue
                                    }
                                },
                                Err(_) => {
                                    print!("{} ", &self.last_token);
                                    err = UndefinedWord;
                                }
                            };
                        }
                    }
            }
            match err {
                NoException => {},
                _ => {
                    self.instruction_pointer = saved_ip;
                    return Some(err)
                }
            }
        }
    }

// High level definitions

    pub fn nest(&mut self) -> Option<Exception> {
        if self.r_stack.len == self.r_stack.cap {
            Some(ReturnStackOverflow)
        } else {
            unsafe {
                ptr::write(self.r_stack.ptr.offset(self.r_stack.len as isize), self.instruction_pointer as isize);
            }
            self.r_stack.len += 1;
            self.instruction_pointer = self.word_list[self.word_pointer].dfa;
            None
        }
    }

    pub fn p_var(&mut self) -> Option<Exception> {
        match self.s_stack.push(self.word_list[self.word_pointer].dfa as isize) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    pub fn p_const(&mut self) -> Option<Exception> {
        match self.s_stack.push(self.s_heap.get_i32(self.word_list[self.word_pointer].dfa) as isize) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    pub fn p_fvar(&mut self) -> Option<Exception> {
        match self.s_stack.push(self.word_list[self.word_pointer].dfa as isize) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    pub fn define(&mut self, action: fn(& mut VM) -> Option<Exception>) -> Option<Exception> {
        self.parse_word();
        match self.find(&self.last_token) {
            Some(_) => print!("Redefining {}", self.last_token),
            None => {}
        }
        if !self.last_token.is_empty() {
            let w = Word::new(self.n_heap.len(), self.last_token.len(), self.s_heap.len(), action);
            self.last_definition = self.word_list.len();
            self.word_list.push (w);
            self.n_heap.push_str(&self.last_token);
            None
        } else {
            self.last_definition = 0;
            Some(UnexpectedEndOfFile)
        }
    }

    pub fn colon(&mut self) -> Option<Exception> {
        match self.define(VM::nest) {
            Some(e) => Some(e),
            None => {
                self.word_list[self.last_definition].hidden = true;
                self.compile()
            }
        }
    }

    pub fn semicolon(&mut self) -> Option<Exception>{
        if self.last_definition != 0 {
            self.s_heap.push_i32(self.idx_exit as i32); 
            self.word_list[self.last_definition].hidden = false;
        }
        self.interpret()
    }

    pub fn create(&mut self) -> Option<Exception> {
        self.define(VM::p_var)
    }

    pub fn variable(&mut self) -> Option<Exception> {
        match self.define(VM::p_var) {
            Some(e) => Some(e),
            None => {
                self.s_heap.push_i32(0);
                None
            }
        }
    }

    pub fn constant(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                match self.define(VM::p_const) {
                    Some(e) => Some(e),
                    None => {
                        self.s_heap.push_i32(v as i32);
                        None
                    }
                }
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn unmark(&mut self) -> Option<Exception> {
        let dfa = self.word_list[self.word_pointer].dfa;
        let nlen = self.s_heap.get_i32(dfa) as usize;
        let wlen = self.s_heap.get_i32(dfa+mem::size_of::<i32>()) as usize;
        let slen = self.s_heap.get_i32(dfa+2*mem::size_of::<i32>()) as usize;
        self.n_heap.truncate(nlen);
        self.word_list.truncate(wlen);
        self.s_heap.truncate(slen);
        None
    }

    pub fn marker(&mut self) -> Option<Exception> {
        self.define(VM::unmark);
        let nlen = self.n_heap.len() as i32;
        let wlen = self.word_list.len() as i32;
        let slen = self.s_heap.len() as i32;
        self.s_heap.push_i32(nlen);
        self.s_heap.push_i32(wlen);
        self.s_heap.push_i32(slen+3*(mem::size_of::<i32>() as i32));
        None
    }

// Control

    pub fn branch(&mut self) -> Option<Exception> {
        self.instruction_pointer = self.s_heap.get_i32(self.instruction_pointer) as usize;
        None
    }

    pub fn zero_branch(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                if v == 0 {
                    self.branch()
                } else {
                    self.instruction_pointer += mem::size_of::<i32>();
                    None
                }
            },
            None => Some(StackUnderflow)
        }
    }

    /// ( n1|u1 n2|u2 -- ) ( R: -- loop-sys )
    ///
    /// Set up loop control parameters with index n2|u2 and limit n1|u1. An
    /// ambiguous condition exists if n1|u1 and n2|u2 are not both the same
    /// type.  Anything already on the return stack becomes unavailable until
    /// the loop-control parameters are discarded. 
    pub fn _do(&mut self) -> Option<Exception> {
        match self.r_stack.push(self.instruction_pointer as isize) {
            Some(_) => Some(ReturnStackOverflow),
            None => {
                self.instruction_pointer += mem::size_of::<i32>();
                self.two_to_r()
            }
        }
    }

    /// Run-time: ( -- ) ( R:  loop-sys1 --  | loop-sys2 )
    ///
    /// An ambiguous condition exists if the loop control parameters are
    /// unavailable. Add one to the loop index. If the loop index is then equal
    /// to the loop limit, discard the loop parameters and continue execution
    /// immediately following the loop. Otherwise continue execution at the
    /// beginning of the loop. 
    pub fn p_loop(&mut self) -> Option<Exception> {
        match self.r_stack.pop2() {
            Some((rn, rt)) => {
                if rt+1 < rn {
                    self.r_stack.push2(rn, rt+1);
                    self.branch()
                } else {
                    match self.r_stack.pop() {
                        Some(_) => {
                            self.instruction_pointer += mem::size_of::<i32>();
                            None
                        },
                        None => Some(ReturnStackUnderflow) 
                    }
                }
            },
            None => Some(ReturnStackUnderflow)
        }
    }

    /// Run-time: ( n -- ) ( R: loop-sys1 -- | loop-sys2 )
    ///
    /// An ambiguous condition exists if the loop control parameters are
    /// unavailable. Add n to the loop index. If the loop index did not cross
    /// the boundary between the loop limit minus one and the loop limit,
    /// continue execution at the beginning of the loop. Otherwise, discard the
    /// current loop control parameters and continue execution immediately
    /// following the loop. 
    pub fn p_plus_loop(&mut self) -> Option<Exception> {
        match self.r_stack.pop2() {
            Some((rn, rt)) => {
                match self.s_stack.pop() {
                    Some(t) => {
                        if rt+t < rn {
                            self.r_stack.push2(rn, rt+t);
                            self.branch()
                        } else {
                            match self.r_stack.pop() {
                                Some(_) => {
                                    self.instruction_pointer += mem::size_of::<i32>();
                                    None
                                },
                                None => Some(ReturnStackUnderflow) 
                            }
                        }
                    },
                    None => Some(StackUnderflow)
                }
            },
            None => Some(ReturnStackUnderflow)
        }
    }

    /// Run-time: ( -- ) ( R: loop-sys -- )
    ///
    /// Discard the loop-control parameters for the current nesting level. An
    /// UNLOOP is required for each nesting level before the definition may be
    /// EXITed. An ambiguous condition exists if the loop-control parameters
    /// are unavailable. 
    pub fn unloop(&mut self) -> Option<Exception> {
        match self.r_stack.pop3() {
            Some(_) => None,
            None => Some(ReturnStackUnderflow)
        }
    }

    pub fn leave(&mut self) -> Option<Exception> {
        match self.r_stack.pop3() {
            Some((third, _, _)) => {
                self.instruction_pointer = self.s_heap.get_i32(third as usize) as usize;
                None
            },
            None => Some(ReturnStackUnderflow)
        }
    }

    pub fn p_i(&mut self) -> Option<Exception> {
        match self.r_stack.last() {
            Some(i) => {
                match self.s_stack.push(i) {
                    Some(_) => Some(StackOverflow),
                    None => None
                }
            },
            None => Some(ReturnStackUnderflow)
        }
    }

    pub fn p_j(&mut self) -> Option<Exception> {
        let pos = self.r_stack.len() - 4;
        match self.r_stack.get(pos) {
            Some(j) => {
                match self.s_stack.push(j) {
                    Some(_) => Some(StackOverflow),
                    None => None
                }
            },
            None => Some(ReturnStackUnderflow)
        }
    }

    pub fn imm_if(&mut self) -> Option<Exception> {
        self.s_heap.push_i32(self.idx_zero_branch as i32);
        self.s_heap.push_i32(0);
        self.here()
    }

    pub fn imm_else(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(if_part) => {
                self.s_heap.push_i32(self.idx_branch as i32);
                self.s_heap.push_i32(0);
                self.here();
                let here = self.s_heap.len();
                self.s_heap.put_i32((if_part - mem::size_of::<i32>() as isize) as usize, here as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn imm_then(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(branch_part) => {
                let here = self.s_heap.len();
                self.s_heap.put_i32((branch_part - mem::size_of::<i32>() as isize) as usize, here as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn imm_begin(&mut self) -> Option<Exception> {
        self.here()
    }

    pub fn imm_while(&mut self) -> Option<Exception> {
        self.s_heap.push_i32(self.idx_zero_branch as i32);
        self.s_heap.push_i32(0);
        self.here()
    }

    pub fn imm_repeat(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((begin_part, while_part)) => {
                self.s_heap.push_i32(self.idx_branch as i32);
                self.s_heap.push_i32(begin_part as i32);
                let here = self.s_heap.len();
                self.s_heap.put_i32((while_part - mem::size_of::<i32>() as isize) as usize, here as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn imm_again(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(begin_part) => {
                self.s_heap.push_i32(self.idx_branch as i32);
                self.s_heap.push_i32(begin_part as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn imm_recurse(&mut self) -> Option<Exception> {
        self.s_heap.push_i32(self.last_definition as i32);
        None
    }

    /// Execution: ( -- a-ddr )
    ///
    /// Append the run-time semantics of _do to the current definition. The semantics are incomplete until resolved by LOOP or +LOOP.
    pub fn imm_do(&mut self) -> Option<Exception> {
        self.s_heap.push_i32(self.idx_do as i32);
        self.s_heap.push_i32(0);
        self.here()
    }

    /// Run-time: ( a-addr -- )
    ///
    /// Append the run-time semantics of _LOOP to the current definition.
    /// Resolve the destination of all unresolved occurrences of LEAVE between
    /// the location given by do-sys and the next location for a transfer of
    /// control, to execute the words following the LOOP. 
    pub fn imm_loop(&mut self) -> Option<Exception>{
        match self.s_stack.pop() {
            Some(do_part) => {
                self.s_heap.push_i32(self.idx_loop as i32);
                self.s_heap.push_i32(do_part as i32);
                let here = self.s_heap.len();
                self.s_heap.put_i32((do_part - mem::size_of::<i32>() as isize) as usize, here as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( a-addr -- )
    ///
    /// Append the run-time semantics of _+LOOP to the current definition.
    /// Resolve the destination of all unresolved occurrences of LEAVE between
    /// the location given by do-sys and the next location for a transfer of
    /// control, to execute the words following +LOOP. 
    pub fn imm_plus_loop(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(do_part) => {
                self.s_heap.push_i32(self.idx_plus_loop as i32);
                self.s_heap.push_i32(do_part as i32);
                let here = self.s_heap.len();
                self.s_heap.put_i32((do_part - mem::size_of::<i32>() as isize) as usize, here as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

// Primitives

    pub fn noop(&mut self) -> Option<Exception> {
        // Do nothing
        None
    }

    /// Run-time: ( -- true )
    ///
    /// Return a true flag, a single-cell value with all bits set. 
    pub fn p_true(&mut self) -> Option<Exception> {
        match self.s_stack.push (-1) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    /// Run-time: ( -- false )
    ///
    /// Return a false flag.
    pub fn p_false(&mut self) -> Option<Exception> {
        match self.s_stack.push (0) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    /// Run-time: (c-addr1 -- c-addr2 )
    ///
    ///Add the size in address units of a character to c-addr1, giving c-addr2. 
    pub fn char_plus(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) =>
                match self.s_stack.push(v + mem::size_of::<u8>() as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: (n1 -- n2 )
    ///
    /// n2 is the size in address units of n1 characters.
    pub fn chars(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) =>
                match self.s_stack.push(v*mem::size_of::<u8>() as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }


    /// Run-time: (a-addr1 -- a-addr2 )
    ///
    /// Add the size in address units of a cell to a-addr1, giving a-addr2.
    pub fn cell_plus(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) =>
                match self.s_stack.push(v + mem::size_of::<i32>() as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: (n1 -- n2 )
    ///
    /// n2 is the size in address units of n1 cells. 
    pub fn cells(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) =>
                match self.s_stack.push(v*mem::size_of::<i32>() as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn lit(&mut self) -> Option<Exception> {
        if self.s_stack.len >= self.s_stack.cap {
            Some(StackOverflow)
        } else {
            unsafe {
                let v = self.s_heap.get_i32(self.instruction_pointer) as isize;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len) as isize), v);
            }
            self.s_stack.len += 1;
            self.instruction_pointer = self.instruction_pointer + mem::size_of::<i32>();
            None
        }
    }

    pub fn swap(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                let t = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)); 
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-2) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), t);
            }
            None
        }
    }

    pub fn dup(&mut self) -> Option<Exception> {
        if self.s_stack.len < 1 {
            Some(StackUnderflow)
        } else if self.s_stack.len >= self.s_stack.cap {
            Some(StackOverflow)
        } else {
            unsafe {
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)));
                self.s_stack.len += 1;
            }
            None
        }
    }

    pub fn p_drop(&mut self) -> Option<Exception> {
        if self.s_stack.len < 1 {
            Some(StackUnderflow)
        } else {
            self.s_stack.len -= 1;
            None
        }
    }

    pub fn nip(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn over(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else if self.s_stack.len >= self.s_stack.cap {
            Some(StackOverflow)
        } else {
            unsafe {
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-2) as isize)));
                self.s_stack.len += 1;
            }
            None
        }
    }

    pub fn rot(&mut self) -> Option<Exception> {
        if self.s_stack.len < 3 {
            Some(StackUnderflow)
        } else {
            unsafe {
                let t = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)); 
                let n = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-2) as isize)); 
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-3) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), t);
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-3) as isize), n);
            }
            None
        }
    }

    pub fn two_drop(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            self.s_stack.len -= 2;
            None
        }
    }

    pub fn two_dup(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else if self.s_stack.len + 2 > self.s_stack.cap {
            Some(StackOverflow)
        } else {
            unsafe {
                self.s_stack.len += 2;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-3) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-4) as isize)));
            }
            None
        }
    }

    pub fn two_swap(&mut self) -> Option<Exception> {
        if self.s_stack.len < 4 {
            Some(StackUnderflow)
        } else {
            unsafe {
                let t = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)); 
                let n = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-2) as isize)); 
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-3) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-4) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-3) as isize), t);
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-4) as isize), n);
            }
            None
        }
    }

    pub fn two_over(&mut self) -> Option<Exception> {
        if self.s_stack.len < 4 {
            Some(StackUnderflow)
        } else if self.s_stack.len + 2 > self.s_stack.cap {
            Some(StackOverflow)
        } else {
            unsafe {
                self.s_stack.len += 2;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-5) as isize)));
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-6) as isize)));
            }
            None
        }
    }

    pub fn depth(&mut self) -> Option<Exception> {
        let len = self.s_stack.len;
        match self.s_stack.push(len as isize) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    pub fn one_plus(&mut self) -> Option<Exception> {
        if self.s_stack.len < 1 {
            Some(StackUnderflow)
        } else {
            unsafe {
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)).wrapping_add(1));
            }
            None
        }
    }

    pub fn one_minus(&mut self) -> Option<Exception> {
        if self.s_stack.len < 1 {
            Some(StackUnderflow)
        } else {
            unsafe {
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize))-1);
            }
            None
        }
    }

    pub fn plus(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize),
                    ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)) + ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn minus(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize),
                    ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)) - ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn star(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize),
                    ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)) * ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn slash(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize),
                    ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)) / ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn p_mod(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                self.s_stack.len -= 1;
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize),
                    ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)) % ptr::read(self.s_stack.ptr.offset((self.s_stack.len) as isize)));
            }
            None
        }
    }

    pub fn slash_mod(&mut self) -> Option<Exception> {
        if self.s_stack.len < 2 {
            Some(StackUnderflow)
        } else {
            unsafe {
                let t = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-1) as isize)); 
                let n = ptr::read(self.s_stack.ptr.offset((self.s_stack.len-2) as isize)); 
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-2) as isize), n%t);
                ptr::write(self.s_stack.ptr.offset((self.s_stack.len-1) as isize), n/t);
            }
            None
        }
    }

    pub fn abs(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(t.abs()) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn negate(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(-t) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn zero_less(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(if t<0 {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn zero_equals(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(if t==0 {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn zero_greater(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(if t>0 {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn zero_not_equals(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(if t!=0 {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn equals(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(if t==n {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn less_than(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(if n<t {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn greater_than(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(if n>t {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn not_equals(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(if n!=t {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn between(&mut self) -> Option<Exception> {
        match self.s_stack.pop3() {
            Some((x1, x2, x3)) =>
                match self.s_stack.push(if x2<=x1 && x1<=x3 {-1} else {0}) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn invert(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(!t) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn and(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(t & n) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn or(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(t | n) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn xor(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(t ^ n) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( x1 u -- x2 )
    ///
    /// Perform a logical left shift of u bit-places on x1, giving x2. Put
    /// zeroes into the least significant bits vacated by the shift. An
    /// ambiguous condition exists if u is greater than or equal to the number
    /// of bits in a cell. 
    pub fn lshift(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(n << t) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( x1 u -- x2 )
    ///
    /// Perform a logical right shift of u bit-places on x1, giving x2. Put
    /// zeroes into the most significant bits vacated by the shift. An
    /// ambiguous condition exists if u is greater than or equal to the number
    /// of bits in a cell. 
    pub fn rshift(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push((n as usize >> t) as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( x1 u -- x2 )
    ///
    /// Perform a arithmetic right shift of u bit-places on x1, giving x2. Put
    /// zeroes into the most significant bits vacated by the shift. An
    /// ambiguous condition exists if u is greater than or equal to the number
    /// of bits in a cell. 
    pub fn arshift(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                match self.s_stack.push(n >> t) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Interpretation: Interpretation semantics for this word are undefined.
    ///
    /// Execution: ( -- ) ( R: nest-sys -- )
    /// Return control to the calling definition specified by nest-sys. Before executing EXIT within a
    /// do-loop, a program shall discard the loop-control parameters by executing UNLOOP. 
    /// TODO: UNLOOP
    pub fn exit(&mut self) -> Option<Exception> {
        if self.r_stack.len == 0 {
            Some(ReturnStackUnderflow)
        } else {
            self.r_stack.len -= 1;
            unsafe {
                self.instruction_pointer = ptr::read(self.r_stack.ptr.offset(self.r_stack.len as isize)) as usize; 
            }
            None
        }
    }

    /// Run-time: ( a-addr -- x )
    ///
    /// x is the value stored at a-addr. 
    pub fn fetch(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(self.s_heap.get_i32(t as usize) as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( x a-addr -- )
    ///
    /// Store x at a-addr. 
    pub fn store(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) => {
                self.s_heap.put_i32(t as usize,  n as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( c-addr -- char )
    ///
    /// Fetch the character stored at c-addr. When the cell size is greater than
    /// character size, the unused high-order bits are all zeroes. 
    pub fn c_fetch(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) =>
                match self.s_stack.push(self.s_heap.get_u8(t as usize) as isize) {
                    Some(_) => Some(StackOverflow),
                    None => None
                },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( char c-addr -- )
    ///
    /// Store char at c-addr. When character size is smaller than cell size,
    /// only the number of low-order bits corresponding to character size are
    /// transferred. 
    pub fn c_store(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) => {
                self.s_heap.put_u8(t as usize,  n as u8);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( "<spaces>name" -- xt )
    ///
    /// Skip leading space delimiters. Parse name delimited by a space. Find
    /// name and return xt, the execution token for name. An ambiguous
    /// condition exists if name is not found. 
    pub fn tick(&mut self) -> Option<Exception> {
        self.parse_word();
        if !self.last_token.is_empty() {
            match self.find(&self.last_token) {
                Some(found_index) =>
                    match self.s_stack.push(found_index as isize) {
                        Some(_) => Some(StackOverflow),
                        None => None
                    },
                None => Some(UndefinedWord)
            }
        } else {
            Some(UnexpectedEndOfFile)
        }
    }

    /// Run-time: ( i*x xt -- j*x )
    ///
    /// Remove xt from the stack and perform the semantics identified by it.
    /// Other stack effects are due to the word EXECUTEd.
    pub fn execute(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(t) => {
                self.execute_word(t as usize)
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( -- addr )
    ///
    /// addr is the data-space pointer. 
    pub fn here(&mut self) -> Option<Exception> {
        match self.s_stack.push(self.s_heap.len() as isize) {
            Some(_) => Some(StackOverflow),
            None => None
        }
    }

    /// Run-time: ( n -- )
    ///
    /// If n is greater than zero, reserve n address units of data space. If n
    /// is less than zero, release |n| address units of data space. If n is
    /// zero, leave the data-space pointer unchanged. 
    pub fn allot(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                unsafe {
                    let len = (self.s_heap.len() as isize + v) as usize;
                    self.s_heap.set_len(len);
                }
                None
            },
            None => Some(StackUnderflow)
        }
    }

    /// Run-time: ( x -- )
    ///
    /// Reserve one cell of data space and store x in the cell. If the
    /// data-space pointer is aligned when , begins execution, it will remain
    /// aligned when , finishes execution. An ambiguous condition exists if the
    /// data-space pointer is not aligned prior to execution of ,. 
    pub fn comma(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                self.s_heap.push_i32(v as i32);
                None
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn p_to_r(&mut self) -> Option<Exception> {
        match self.s_stack.pop() {
            Some(v) => {
                if self.r_stack.len >= self.r_stack.cap {
                    Some(ReturnStackOverflow)
                } else {
                    unsafe {
                        ptr::write(self.r_stack.ptr.offset(self.r_stack.len as isize), v);
                    }
                    self.r_stack.len += 1;
                    None
                }
            },
            None => Some(StackUnderflow)
        }
    }

    pub fn r_from(&mut self) -> Option<Exception> {
        if self.r_stack.len == 0 {
            Some(ReturnStackUnderflow)
        } else {
            self.r_stack.len -= 1;
            unsafe {
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset(self.r_stack.len as isize))); 
            }
            None
        }
    }

    pub fn r_fetch(&mut self) -> Option<Exception> {
        if self.r_stack.len == 0 {
            Some(ReturnStackUnderflow)
        } else {
            unsafe {
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset((self.r_stack.len-1) as isize))); 
            }
            None
        }
    }

    pub fn two_to_r(&mut self) -> Option<Exception> {
        match self.s_stack.pop2() {
            Some((n,t)) =>
                if self.r_stack.len >= self.r_stack.cap-1 {
                    Some(ReturnStackOverflow)
                } else {
                    unsafe {
                        ptr::write(self.r_stack.ptr.offset(self.r_stack.len as isize), n);
                        ptr::write(self.r_stack.ptr.offset((self.r_stack.len+1) as isize), t);
                    }
                    self.r_stack.len += 2;
                    None
                },
            None => Some(StackUnderflow)
        }
    }

    pub fn two_r_from(&mut self) -> Option<Exception> {
        if self.r_stack.len < 2 {
            Some(ReturnStackUnderflow)
        } else {
            self.r_stack.len -= 2;
            unsafe {
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset(self.r_stack.len as isize))); 
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset((self.r_stack.len+1) as isize))); 
            }
            None
        }
    }

    pub fn two_r_fetch(&mut self) -> Option<Exception> {
        if self.r_stack.len < 2 {
            Some(ReturnStackUnderflow)
        } else {
            unsafe {
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset((self.r_stack.len-2) as isize))); 
                self.s_stack.push(ptr::read(self.r_stack.ptr.offset((self.r_stack.len-1) as isize))); 
            }
            None
        }
    }

    pub fn pause(&mut self) -> Option<Exception> {
        if self.r_stack.len >= self.r_stack.cap {
            Some(ReturnStackOverflow)
        } else {
            unsafe {
                ptr::write(self.r_stack.ptr.offset(self.r_stack.len as isize), self.instruction_pointer as isize);
            }
            self.r_stack.len += 1;
            self.instruction_pointer = 0;
            self.is_paused = true;
            None
        }
    }

// Error handlling

    #[inline(never)]
    pub fn abort(&mut self) -> Option<Exception> {
        self.s_stack.clear();
        self.f_stack.clear();
        self.quit();
        Some(Abort)
    }

    #[inline(never)]
    pub fn quit(&mut self) -> Option<Exception> {
        self.r_stack.len = 0;
        self.input_buffer.clear();
        self.source_index = 0;
        self.instruction_pointer = 0;
        self.last_definition = 0;
        self.is_paused = false;
        self.interpret();
        None
    }

    #[inline(never)]
    fn bye(&mut self) -> Option<Exception> {
        Some(Bye)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::test::Bencher;
    use std::mem;
    use exception::Exception::Abort;

    #[bench]
    fn bench_noop (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        b.iter(|| vm.noop());
    }

    #[test]
    fn test_find() {
        let vm = &mut VM::new(1024);
        assert!(vm.find("").is_none());
        assert!(vm.find("word-not-exist").is_none());
        assert_eq!(0usize, vm.find("noop").unwrap());
    }

    #[bench]
    fn bench_find_word_not_exist(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        b.iter(|| vm.find("unknown"));
    }

    #[bench]
    fn bench_find_word_at_beginning_of_wordlist(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        b.iter(|| vm.find("noop"));
    }

    #[bench]
    fn bench_find_word_at_end_of_wordlist(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        b.iter(|| vm.find("bye"));
    }

    #[test]
    fn test_inner_interpreter_without_nest () {
        let vm = &mut VM::new(1024);
        let ip = vm.s_heap.len();
        vm.compile_integer(3);
        vm.compile_integer(2);
        vm.compile_integer(1);
        vm.inner_interpret(ip);
        assert_eq!(3usize, vm.s_stack.len());
    }

    #[bench]
    fn bench_inner_interpreter_without_nest (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        let ip = vm.s_heap.len();
        let idx = 0; // NOP
        vm.compile_word(idx);
        vm.compile_word(idx);
        vm.compile_word(idx);
        vm.compile_word(idx);
        vm.compile_word(idx);
        vm.compile_word(idx);
        vm.compile_word(idx);
        b.iter(|| vm.inner_interpret(ip));
    }

    #[test]
    fn test_drop() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        assert!(vm.p_drop().is_none());
        assert!(vm.s_stack.is_empty());
    }

    #[bench]
    fn bench_drop(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.p_drop();
            vm.s_stack.push(1);
        });
    }

    #[test]
    fn test_nip() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.nip().is_none());
        assert!(vm.s_stack.len()==1);
        assert!(vm.s_stack.last() == Some(2));
    }

    #[bench]
    fn bench_nip(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.nip();
            vm.s_stack.push(1);
        });
    }

    #[test]
    fn test_swap () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.swap().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[bench]
    fn bench_swap (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        b.iter(|| vm.swap());
    }

    #[test]
    fn test_dup () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        assert!(vm.dup().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(1));
    }

    #[bench]
    fn bench_dup (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.dup();
            vm.s_stack.pop();
        });
    }

    #[test]
    fn test_over () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.over().is_none());
        assert_eq!(vm.s_stack.len(), 3);
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
    }

    #[bench]
    fn bench_over (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        b.iter(|| {
            vm.over();
            vm.s_stack.pop();
        });
    }

    #[test]
    fn test_rot () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        assert!(vm.rot().is_none());
        assert_eq!(vm.s_stack.len(), 3);
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(3));
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[bench]
    fn bench_rot (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        b.iter(|| vm.rot());
    }

    #[test]
    fn test_2drop () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.two_drop().is_none());
        assert!(vm.s_stack.is_empty());
    }

    #[bench]
    fn bench_2drop (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        b.iter(|| {
            vm.s_stack.push(1);
            vm.s_stack.push(2);
            vm.two_drop();
        });
    }

    #[test]
    fn test_2dup () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.two_dup().is_none());
        assert_eq!(vm.s_stack.len(), 4);
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
    }

    #[bench]
    fn bench_2dup (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        b.iter(|| {
            vm.two_dup();
            vm.two_drop();
        });
    }

    #[test]
    fn test_2swap () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        vm.s_stack.push(4);
        assert!(vm.two_swap().is_none());
        assert_eq!(vm.s_stack.len(), 4);
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(4));
        assert_eq!(vm.s_stack.pop(), Some(3));
    }

    #[bench]
    fn bench_2swap (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        vm.s_stack.push(4);
        b.iter(|| vm.two_swap());
    }

    #[test]
    fn test_2over () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        vm.s_stack.push(4);
        assert!(vm.two_over().is_none());
        assert_eq!(vm.s_stack.len(), 6);
        assert_eq!(vm.s_stack.as_slice(), [1, 2, 3, 4, 1, 2]);
    }

    #[bench]
    fn bench_2over (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        vm.s_stack.push(3);
        vm.s_stack.push(4);
        b.iter(|| {
            vm.two_over();
            vm.two_drop();
        });
    }

    #[test]
    fn test_depth() {
        let vm = &mut VM::new(1024);
        vm.depth();
        vm.depth();
        vm.depth();
        assert_eq!(vm.s_stack.as_slice(), [0, 1, 2]);
    }

    #[test]
    fn test_one_plus() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        assert!(vm.one_plus().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[bench]
    fn bench_one_plus(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        b.iter(|| {
            vm.one_plus();
        });
    }

    #[test]
    fn test_one_minus() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(2);
        assert!(vm.one_minus().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(1));
    }

    #[bench]
    fn bench_one_minus(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        b.iter(|| {
            vm.one_minus();
        });
    }

    #[test]
    fn test_minus() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(5);
        vm.s_stack.push(7);
        assert!(vm.minus().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-2));
    }

    #[bench]
    fn bench_minus(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        b.iter(|| {
            vm.dup();
            vm.minus();
        });
    }

    #[test]
    fn test_plus() {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(5);
        vm.s_stack.push(7);
        assert!(vm.plus().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(12));
    }

    #[bench]
    fn bench_plus(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.dup();
            vm.plus();
        });
    }

    #[test]
    fn test_star () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(5);
        vm.s_stack.push(7);
        assert!(vm.star().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(35));
    }

    #[bench]
    fn bench_star(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.dup();
            vm.star();
        });
    }

    #[test]
    fn test_slash () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(30);
        vm.s_stack.push(7);
        assert!(vm.slash().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
    }

    #[bench]
    fn bench_slash(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        b.iter(|| {
            vm.dup();
            vm.slash();
        });
    }

    #[test]
    fn test_mod () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(30);
        vm.s_stack.push(7);
        assert!(vm.p_mod().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[bench]
    fn bench_mod(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        b.iter(|| {
            vm.p_mod();
            vm.s_stack.push(2);
        });
    }

    #[test]
    fn test_slash_mod () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(30);
        vm.s_stack.push(7);
        assert!(vm.slash_mod().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(4));
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[bench]
    fn bench_slash_mod(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.s_stack.push2(1, 2);
        b.iter(|| {
            vm.slash_mod();
            vm.p_drop();
            vm.s_stack.push(2)
        });
    }

    #[test]
    fn test_abs () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(-30);
        assert!(vm.abs().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(30));
    }

    #[test]
    fn test_negate () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(30);
        assert!(vm.negate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-30));
    }

    #[test]
    fn test_zero_less () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(-1);
        assert!(vm.zero_less().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(0);
        assert!(vm.zero_less().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_zero_equals () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        assert!(vm.zero_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(-1);
        assert!(vm.zero_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.s_stack.push(1);
        assert!(vm.zero_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_zero_greater () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        assert!(vm.zero_greater().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(0);
        assert!(vm.zero_greater().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_zero_not_equals () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        assert!(vm.zero_not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.s_stack.push(-1);
        assert!(vm.zero_not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(1);
        assert!(vm.zero_not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
    }

    #[test]
    fn test_less_than () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(-1);
        vm.s_stack.push(0);
        assert!(vm.less_than().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(0);
        vm.s_stack.push(0);
        assert!(vm.less_than().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_equals () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        vm.s_stack.push(0);
        assert!(vm.equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(-1);
        vm.s_stack.push(0);
        assert!(vm.equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.s_stack.push(1);
        vm.s_stack.push(0);
        assert!(vm.equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_greater_than () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(0);
        assert!(vm.greater_than().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(0);
        vm.s_stack.push(0);
        assert!(vm.greater_than().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_not_equals () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(0);
        vm.s_stack.push(0);
        assert!(vm.not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.s_stack.push(-1);
        vm.s_stack.push(0);
        assert!(vm.not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(1);
        vm.s_stack.push(0);
        assert!(vm.not_equals().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
    }

    #[test]
    fn test_between () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.between().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(1);
        vm.s_stack.push(0);
        vm.s_stack.push(1);
        assert!(vm.between().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
        vm.s_stack.push(0);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.between().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.s_stack.push(3);
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.between().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_invert () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(707);
        assert!(vm.invert().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-708));
    }

    #[test]
    fn test_and () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(707);
        vm.s_stack.push(007);
        assert!(vm.and().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(3));
    }

    #[test]
    fn test_or () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(707);
        vm.s_stack.push(07);
        assert!(vm.or().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(711));
    }

    #[test]
    fn test_xor () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(707);
        vm.s_stack.push(07);
        assert!(vm.xor().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(708));
    }

    #[test]
    fn test_lshift () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(1);
        vm.s_stack.push(1);
        assert!(vm.lshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(2));
        vm.s_stack.push(1);
        vm.s_stack.push(2);
        assert!(vm.lshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
    }

    #[test]
    fn test_rshift () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(8);
        vm.s_stack.push(1);
        assert!(vm.rshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
        vm.s_stack.push(-1);
        vm.s_stack.push(1);
        assert!(vm.rshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert!(vm.s_stack.pop().unwrap() > 0);
    }

    #[test]
    fn test_arshift () {
        let vm = &mut VM::new(1024);
        vm.s_stack.push(8);
        vm.s_stack.push(1);
        assert!(vm.arshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
        vm.s_stack.push(-8);
        vm.s_stack.push(1);
        assert!(vm.arshift().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-4));
    }

    #[test]
    fn test_parse_word () {
        let vm = &mut VM::new(1024);
        vm.set_source("hello world\t\r\n\"");
        assert!(vm.parse_word().is_none());
        assert_eq!(vm.last_token, "hello");
        assert_eq!(vm.source_index, 6);
        assert!(vm.parse_word().is_none());
        assert_eq!(vm.last_token, "world");
        assert_eq!(vm.source_index, 12);
        assert!(vm.parse_word().is_none());
        assert_eq!(vm.last_token, "\"");
    }

    #[test]
    fn test_evaluate () {
        let vm = &mut VM::new(1024);
        vm.set_source("false true dup 1+ 2 -3");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 5);
        assert_eq!(vm.s_stack.pop(), Some(-3));
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(0));
        assert_eq!(vm.s_stack.pop(), Some(-1));
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[bench]
    fn bench_compile_words_at_beginning_of_wordlist (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.set_source("marker empty");
        assert!(vm.evaluate().is_none());
        b.iter(|| {
            vm.set_source(": main noop noop noop noop noop noop noop noop ; empty");
            vm.evaluate();
            vm.s_stack.clear();
        });
    }

    #[bench]
    fn bench_compile_words_at_end_of_wordlist(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.set_source("marker empty");
        vm.evaluate();
        b.iter(|| {
            vm.set_source(": main bye bye bye bye bye bye bye bye ; empty");
            vm.evaluate();
            vm.s_stack.clear();
        });
    }

    #[test]
    fn test_colon_and_semi_colon() {
        let vm = &mut VM::new(1024);
        vm.set_source(": 2+3 2 3 + ; 2+3");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(5));
    }

    #[test]
    fn test_constant () {
        let vm = &mut VM::new(1024);
        vm.set_source("5 constant x x x");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(5));
        assert_eq!(vm.s_stack.pop(), Some(5));
    }

    #[test]
    fn test_variable_and_store_fetch () {
        let vm = &mut VM::new(1024);
        vm.set_source("variable x  x @  3 x !  x @");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(3));
        assert_eq!(vm.s_stack.pop(), Some(0));
    }

    #[test]
    fn test_char_plus_and_chars() {
        let vm = &mut VM::new(1024);
        vm.set_source("2 char+  9 chars");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.as_slice(), [3, 9]);
    }

    #[test]
    fn test_cell_plus_and_cells() {
        let vm = &mut VM::new(1024);
        vm.set_source("2 cell+  9 cells");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.as_slice(), [6, 36]);
    }

    #[test]
    fn test_execute () {
        let vm = &mut VM::new(1024);
        vm.set_source("1 2  ' swap execute");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.s_stack.pop(), Some(2));
    }

    #[test]
    fn test_here_allot () {
        let vm = &mut VM::new(1024);
        vm.set_source("here 2 cells allot here -");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-((mem::size_of::<i32>()*2) as isize)));
    }

    #[test]
    fn test_here_comma_compile_interpret () {
        let vm = &mut VM::new(1024);
        vm.set_source("here 1 , 2 , ] noop execute [ here");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop(), Some(20));
        assert_eq!(vm.s_stack.pop(), Some(4));
        assert_eq!(vm.s_heap.get_i32(0), 0);
        assert_eq!(vm.s_heap.get_i32(4), 1);
        assert_eq!(vm.s_heap.get_i32(8), 2);
        assert_eq!(vm.s_heap.get_i32(12), 0);
        assert_eq!(vm.s_heap.get_i32(16), 1);
    }

    #[test]
    fn test_to_r_r_fetch_r_from () {
        let vm = &mut VM::new(1024);
        vm.set_source(": t 3 >r 2 r@ + r> + ; t");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(8));
    }

    #[bench]
    fn bench_to_r_r_fetch_r_from (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 3 >r r@ drop r> drop ;");
        vm.evaluate();
        vm.set_source("' main");
        vm.evaluate();
        b.iter(|| {
            vm.dup();
            vm.execute();
            vm.inner();
        });
    }

    #[test]
    fn test_two_to_r_two_r_fetch_two_r_from () {
        let vm = &mut VM::new(1024);
        vm.set_source(": t 1 2 2>r 2r@ + 2r> - * ; t");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-3));
    }

    #[bench]
    fn bench_two_to_r_two_r_fetch_two_r_from (b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 1 2 2>r 2r@ 2drop 2r> 2drop ;");
        vm.evaluate();
        vm.set_source("' main");
        vm.evaluate();
        b.iter(|| {
            vm.dup();
            vm.execute();
            vm.inner();
        });
    }

    #[test]
    fn test_if_else_then () {
        let vm = &mut VM::new(1024);
        vm.set_source(": t1 0 if true else false then ; t1");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(0));
        vm.set_source(": t2 1 if true else false then ; t2");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
    }

    #[test]
    fn test_begin_again () {
        let vm = &mut VM::new(1024);
        vm.set_source(": t1 0 begin 1+ dup 3 = if exit then again ; t1");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(3));
    }

    #[test]
    fn test_begin_while_repeat () {
        let vm = &mut VM::new(1024);
        vm.set_source(": t1 0 begin 1+ dup 3 <> while repeat ; t1");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(3));
    }

    #[test]
    fn test_backlash () {
        let vm = &mut VM::new(1024);
        vm.set_source("1 2 3 \\ 5 6 7");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 3);
        assert_eq!(vm.s_stack.pop(), Some(3));
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
    }

    #[test]
    fn test_marker_unmark () {
        let vm = &mut VM::new(1024);
        vm.set_source("marker empty here empty here =");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(-1));
    }

    #[test]
    fn test_quit () {
        let vm = &mut VM::new(1024);
        vm.set_source("1 2 3 quit 5 6 7");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 3);
        assert_eq!(vm.s_stack.pop(), Some(3));
        assert_eq!(vm.s_stack.pop(), Some(2));
        assert_eq!(vm.s_stack.pop(), Some(1));
        assert_eq!(vm.input_buffer.len(), 0);
    }

    #[test]
    fn test_abort () {
        let vm = &mut VM::new(1024);
        vm.set_source("1 2 3 abort 5 6 7");
        match vm.evaluate() {
            Some(Abort) => assert!(true),
            _ => assert!(false)
        }
        assert_eq!(vm.s_stack.len(), 0);
    }

    #[bench]
    fn bench_fib(b: &mut Bencher) {
        let vm = &mut VM::new(1024);
        vm.set_source(": fib dup 2 < if drop 1 else dup 1- recurse swap 2 - recurse + then ;");
        assert!(vm.evaluate().is_none());
        vm.set_source(": main 7 fib drop ;");
        vm.evaluate();
        vm.set_source("' main");
        vm.evaluate();
        b.iter(|| {
            vm.dup();
            vm.execute();
            vm.inner();
        });
    }

    #[test]
    fn test_do_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 1 5 0 do 1+ loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(6));
    }

    #[test]
    fn test_do_unloop_exit_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 1 5 0 do 1+ dup 3 = if unloop exit then loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(3));
    }

    #[test]
    fn test_do_plus_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 1 5 0 do 1+ 2 +loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
        vm.set_source(": main 1 6 0 do 1+ 2 +loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 1);
        assert_eq!(vm.s_stack.pop(), Some(4));
    }

    #[test]
    fn test_do_leave_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 1 5 0 do 1+ dup 3 = if drop 88 leave then loop 9 ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 2);
        assert_eq!(vm.s_stack.pop2(), Some((88, 9)));
    }

    #[test]
    fn test_do_i_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 3 0 do i loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 3);
        assert_eq!(vm.s_stack.pop3(), Some((0, 1, 2)));
    }

    #[test]
    fn test_do_i_j_loop() {
        let vm = &mut VM::new(1024);
        vm.set_source(": main 6 4 do 3 1 do i j * loop loop ;  main");
        assert!(vm.evaluate().is_none());
        assert_eq!(vm.s_stack.len(), 4);
        assert_eq!(vm.s_stack.as_slice(), [4, 8, 5, 10]);
    }
}
