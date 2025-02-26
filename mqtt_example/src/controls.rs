use crate::event::DeviceEvent;
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::gpio::{Input, InputPin, InterruptType, PinDriver};
use esp_idf_svc::hal::task::notification::Notification;
use std::num::NonZeroU32;
use std::sync::mpsc;
use std::thread;
use std::thread::{Scope, ScopedJoinHandle};
use std::time::Duration;

pub struct Controls<'controls, SELECT, ENTER>
where
    SELECT: InputPin,
    ENTER: InputPin,
{
    btn_select: PinDriver<'controls, SELECT, Input>,
    btn_enter: PinDriver<'controls, ENTER, Input>,

    selection: u8,
}

impl<'controls, SELECT: InputPin, ENTER: InputPin> Controls<'controls, SELECT, ENTER> {
    pub fn new(pin_select: SELECT, pin_enter: ENTER) -> anyhow::Result<Self> {
        let mut btn_select = PinDriver::input(pin_select)?;
        let mut btn_enter = PinDriver::input(pin_enter)?;

        btn_select.set_interrupt_type(InterruptType::NegEdge)?;
        btn_enter.set_interrupt_type(InterruptType::NegEdge)?;

        Ok(Self {
            btn_select,
            btn_enter,
            selection: 0,
        })
    }

    pub fn spawn_thread<'scope>(
        mut self,
        scope: &'scope Scope<'scope, '_>,
        sender: mpsc::Sender<DeviceEvent>,
    ) -> Result<ScopedJoinHandle<'scope, ()>, std::io::Error>
    where
        'controls: 'scope,
    {
        thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(scope, move || {
                let notification = Notification::new();
                let notifier_select = notification.notifier();
                let notifier_enter = notification.notifier();

                // Usage of interrupts is currently unsafe.
                unsafe {
                    self.btn_select
                        .subscribe(move || {
                            notifier_select.notify_and_yield(NonZeroU32::new(1).unwrap());
                        })
                        .unwrap();
                    self.btn_enter
                        .subscribe(move || {
                            notifier_enter.notify_and_yield(NonZeroU32::new(2).unwrap());
                        })
                        .unwrap();
                }
                loop {
                    self.enable_interrupts().unwrap();
                    let btn_num = notification.wait(delay::BLOCK).unwrap().get();
                    match btn_num {
                        1 => {
                            self.selection += 1;
                            if self.selection > 3 {
                                self.selection = 0;
                            }
                            sender
                                .send(DeviceEvent::Select {
                                    data: self.selection,
                                })
                                .unwrap();
                        }
                        2 => {
                            sender
                                .send(DeviceEvent::Enter {
                                    data: self.selection,
                                })
                                .unwrap();
                            // GPIO35 has no pull up resistor, this helps not to send multiple events
                            thread::sleep(Duration::from_millis(500))
                        }
                        _ => {}
                    }
                }
            })
    }

    fn enable_interrupts(&mut self) -> anyhow::Result<()> {
        self.btn_select.enable_interrupt()?;
        self.btn_enter.enable_interrupt()?;
        Ok(())
    }
}
