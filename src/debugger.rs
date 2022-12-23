pub struct Debugger{
    pub show_gui: bool,
    pub init_cpu: bool,
}
impl Default for Debugger {
    fn default() -> Self {
        Debugger { show_gui: true
            , init_cpu: true }
    }
}