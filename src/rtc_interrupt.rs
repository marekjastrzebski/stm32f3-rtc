#[derive(Clone)]
pub enum RtcInterruptOutputPolarity {
    High = 0,
    Low = 1,
}

impl Into<bool> for RtcInterruptOutputPolarity {
    fn into(self) -> bool {
        (self as u8) == 1
    }
}

#[derive(Clone)]
pub enum RtcInterruptOutputSelection {
    Disabled = 0b00,
    AlarmA = 0b01,
    AlarmB = 0b10,
    WakeUp = 0b11,
}

impl Into<u8> for RtcInterruptOutputSelection {
    fn into(self) -> u8 {
        self as u8
    }
}


pub struct RtcInterrupt {
    pub(crate) output_selection: RtcInterruptOutputSelection,
    pub(crate) polarity: RtcInterruptOutputPolarity,
}

impl RtcInterrupt {
    pub fn new() -> Self {
        RtcInterrupt {
            output_selection: RtcInterruptOutputSelection::Disabled,
            polarity: RtcInterruptOutputPolarity::High,
        }
    }

    pub fn set_output_selection(mut self, sel: RtcInterruptOutputSelection) -> Self {
        self.output_selection = sel;
        self
    }

    pub fn set_polarity(mut self, pol: RtcInterruptOutputPolarity) -> Self {
        self.polarity = pol;
        self
    }
}
