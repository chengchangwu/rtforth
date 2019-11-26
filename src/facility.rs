use core::Core;

pub trait Facility: Core {
    /// Run-time: ( --  )
    ///
    /// Add facility primitives.
    fn add_facility(&mut self) {
        self.add_primitive("mtime", Facility::mtime);
        self.add_primitive("utime", Facility::utime);
    }

    /// System time in milli-seconds. `mtime ( -- milli-seconds )`
    primitive! {fn mtime(&mut self) {
        let now = self.system_time_ns() / 1_000_000;
        self.s_stack().push(now as isize);
    }}

    /// System time in micro-seconds. `utime ( -- micro-seconds )`
    primitive! {fn utime(&mut self) {
        let now = self.system_time_ns() / 1_000;
        self.s_stack().push(now as isize);

    }}
}
