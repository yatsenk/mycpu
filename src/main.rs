use sysinfo::{
    System, 
    CpuRefreshKind, 
    RefreshKind, 
    Components,
};

use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Span, Line};
use ratatui::widgets::{Paragraph, Block, Tabs, Wrap, ListState, ListItem, List, LineGauge};
use ratatui::{DefaultTerminal, Frame};
use ratatui::symbols;

const LOGS: [(&str, &str); 26] = [
    ("Event1", "INFO"),
    ("Event2", "INFO"),
    ("Event3", "CRITICAL"),
    ("Event4", "ERROR"),
    ("Event5", "INFO"),
    ("Event6", "INFO"),
    ("Event7", "WARNING"),
    ("Event8", "INFO"),
    ("Event9", "INFO"),
    ("Event10", "INFO"),
    ("Event11", "CRITICAL"),
    ("Event12", "INFO"),
    ("Event13", "INFO"),
    ("Event14", "INFO"),
    ("Event15", "INFO"),
    ("Event16", "INFO"),
    ("Event17", "ERROR"),
    ("Event18", "ERROR"),
    ("Event19", "INFO"),
    ("Event20", "INFO"),
    ("Event21", "WARNING"),
    ("Event22", "INFO"),
    ("Event23", "INFO"),
    ("Event24", "WARNING"),
    ("Event25", "INFO"),
    ("Event26", "INFO"),
];

fn main() -> Result<()> {
    color_eyre::install()?;
    let tick_rate = std::time::Duration::from_millis(1000);
    ratatui::run(|terminal| App::new().run(terminal, tick_rate))
}

#[derive(Debug)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

#[derive(Debug, Default)]
struct TabsState<'a> {
    titles: Vec<&'a str>,
    index: usize,
}

impl<'a> TabsState<'a> {
    const fn new(titles: Vec<&'a str>) -> Self {
        Self { titles, index: 0 }
    }

    fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

#[derive(Debug)]
struct Cpu {
    sys: System,
    comps: Components,
}

impl Cpu {
    fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
        );

        let comps = Components::new_with_refreshed_list();

        Self {
            sys,
            comps,
        }
    }

    fn get_usage(&mut self) -> f32 {
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        self.sys.refresh_cpu_all();  
        self.sys.global_cpu_usage()
    }

    fn get_frequency(&mut self) -> u64 {
        self.sys.refresh_cpu_all();
        self.sys.cpus().iter().map(|cpu| cpu.frequency()).sum()
    }

    fn get_max_frequency(&mut self) -> u64 {
        60000
    }

    fn get_name(&mut self) -> String {
        self.sys.cpus().iter().map(|cpu| cpu.name()).collect()
    }

    fn get_brand(&mut self) -> String {
        let names: Vec<&str> = self.sys.cpus().iter().map(|cpu| cpu.brand()).collect();
        names[0].to_string()
    }

    fn get_vendor_id(&mut self) -> String {
        let ids: Vec<&str> = self.sys.cpus().iter().map(|cpu| cpu.vendor_id()).collect();
        ids[0].to_string()
    }

    fn get_cpu_temp(&mut self) -> f32 {
        for comp in &self.comps {
            if let Some(temperature) = comp.temperature() {
                return temperature;
            }
        }

        return 0.0;
    }
} 


#[derive(Debug)]
struct App<'a> {
    title: &'a str,
    tabs: TabsState<'a>,
    logs: StatefulList<(&'a str, &'a str)>,
    cpu: Cpu,
    usage: f32,
    frequency: u64,
    max_frequency: u64,
    name: String,
    brand: String,
    vendor_id: String,
    temperature: f32,
}

impl<'a> App<'a> {
    fn new() -> Self {
        let mut cpu = Cpu::new();
        let max_frequency = cpu.get_max_frequency();
        let frequency = cpu.get_frequency();
        let name = cpu.get_name();
        let brand = cpu.get_brand();
        let vendor_id = cpu.get_vendor_id();
        let temperature = cpu.get_cpu_temp();
        Self {
            title: "MyCPU",
            tabs: TabsState::new(vec!["Power", "Info", "Other"]),
            logs: StatefulList::with_items(LOGS.to_vec()),
            cpu,
            usage: 0.0,
            frequency,
            max_frequency,
            name,
            brand, 
            vendor_id,
            temperature,
        }
    }

    fn on_right(&mut self) {
        self.tabs.next();
    }

    fn on_left(&mut self) {
        self.tabs.previous();
    }

    fn run(mut self, terminal: &mut DefaultTerminal, tick_rate: std::time::Duration) -> Result<()> {
        let mut last_tick = std::time::Instant::now();
        loop {
            terminal.draw(|frame| self.render(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if !event::poll(timeout)? {
                self.on_tick();
                last_tick = std::time::Instant::now();
                continue;
            }

            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Tab => self.on_right(),
                    KeyCode::BackTab => self.on_left(),
                    _ => {}   
                }
            }
        }
    }
    
    fn render(&mut self, frame: &mut Frame) { 
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(frame.area());
        let tabs = self
            .tabs
            .titles
            .iter()
            .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::White))))
            .collect::<Tabs>()
            .block(Block::bordered().style(Color::White).title(self.title))
            .highlight_style(Style::default().fg(Color::LightBlue))
            .select(self.tabs.index);
        frame.render_widget(tabs, chunks[0]);
        match self.tabs.index {
            0 => self.render_first_tab(frame, chunks[1]),
            _ => {}
        };
    }

    fn render_first_tab(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::horizontal([
            Constraint::Length(70),
            Constraint::Min(5),
            Constraint::Length(20),
        ])
        .split(area);

        self.draw_gauge(frame, chunks[0]);
        self.draw_text(frame, chunks[1]);
        self.draw_logs(frame, chunks[2]);
    }

    fn draw_logs(&mut self, frame: &mut Frame, area: Rect) {
        let info_style = Style::default().fg(Color::Blue);
        let warning_style = Style::default().fg(Color::Yellow);
        let error_style = Style::default().fg(Color::Red);
        let critical_style = Style::default().fg(Color::Green);
        let logs: Vec<ListItem> = self
            .logs
            .items
            .iter()
            .map(|&(evt, level)| {
                let s = match level {
                    "ERROR" => error_style,
                    "CRITICAL" => critical_style,
                    "WARNING" => warning_style,
                    _ => info_style,
                };
                let content = vec![Line::from(vec![
                    Span::styled(format!("{level:<9}"), s),
                    Span::raw(evt),
                ])];
                ListItem::new(content)
            })
            .collect();
        let logs = List::new(logs).block(Block::bordered());
        frame.render_stateful_widget(logs, area, &mut self.logs.state);
    }

    fn draw_gauge(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .horizontal_margin(2)
        .split(area);

        let block = Block::bordered();
        frame.render_widget(block, area);
        
        let usage_gauge = LineGauge::default()
            .block(Block::new().title(format!("Usage: {}%", self.usage.round())))
            .filled_style(Style::default().fg(Color::Magenta))
            .filled_symbol(symbols::line::THICK_HORIZONTAL)
            .unfilled_symbol(symbols::line::THICK_HORIZONTAL)
            .label("")
            .ratio(self.usage as f64 / 100.0);
        frame.render_widget(usage_gauge, chunks[1]);

        let ratio =  self.frequency / self.max_frequency;
        let frequency_gauge = LineGauge::default()
            .block(Block::new().title(format!("Frequency: {} MHz", self.frequency)))
            .filled_style(Style::default().fg(Color::Magenta))
            .filled_symbol(symbols::line::THICK_HORIZONTAL)
            .unfilled_symbol(symbols::line::THICK_HORIZONTAL)
            .label("")
            .ratio(ratio as f64);
        frame.render_widget(frequency_gauge, chunks[2]);
    }

    fn draw_text(&self, frame: &mut Frame, area: Rect) {
        let text = vec![
            Line::from(vec![
                Span::from("Name: "),
                Span::styled(self.brand.clone(), Style::default().fg(Color::Rgb(255, 165, 0))),
            ]),
            Line::from(vec![
                Span::from("Vendor ID: "),
                Span::styled(self.vendor_id.clone(), Style::default().fg(Color::Rgb(255, 165, 0))),
            ]),
            Line::from(vec![
                Span::from("Temperature: "),
                Span::styled(self.temperature.to_string(), Style::default().fg(Color::Rgb(255, 165, 0))),
            ]),
        ];
        let paragraph = Paragraph::new(text).block(Block::bordered()).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn on_tick(&mut self) {
        self.usage = self.cpu.get_usage();
        self.frequency = self.cpu.get_frequency();

        let log = self.logs.items.pop().unwrap();
        self.logs.items.insert(0, log);
    }
}
