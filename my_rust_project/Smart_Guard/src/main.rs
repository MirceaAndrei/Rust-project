#![no_std]
#![no_main]
#![allow(dead_code)]
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_rp::gpio::{self, Input, Output, Pull};
use embassy_time::{Duration, Timer};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use cortex_m::peripheral::Peripherals;
use cortex_m::delay::Delay as CortexDelay; 
use ufmt_write::uWrite;
use lcd_lcm1602_i2c::Lcd;
use embassy_rp::i2c::{I2c, Config};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver};

const LED_PIN: u8 = 16; // GPIO pin connected to the  red LED
const LED_PIN_2: u8 = 17; // GPIO pin connected to the green LED2
const LED_PIN_3: u8 = 18; // GPIO pin connected to the blue LED3
const BUZZER_PIN: u8 = 1; // GPIO pin connected to the buzzer
const PIR_PIN: u8 = 3; // GPIO pin connected to the PIR sensor
const SCL_PIN: u8 = 13;// GPIO pin connected to the  scl pin of the lcd
const SDA_PIN: u8 = 12; //GPIO pin connected to the sda pin of the lcd
const MAX_PASSWORD_ATTEMPTS: u8 = 3;
const COL_PINS: [u8; 4] = [4, 5, 6, 7]; // GPIO pins connected to the columns
const KEYPAD_MAPPING: [char; 4] = ['1', '2', '3', '4']; // Mapping of the keypad buttons to the characters
const LCD_ADDRESS: u8 = 0x27; // Address of the LCD

#[embassy_executor::task]

async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]

async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut entered_password: [char; 4] = ['0', '0', '0', '0']; // The entered password
    let mut incorrect_attempts: u8 = 0;//Incorrect password attempts counter
    let mut correct_password: [char; 4] = ['0', '0', '0', '0'];// The array for the correct password

    //The lcd part:

    let scl = p.PIN_13;// scl pin of the lcd
    let sda = p.PIN_12;// sda pin of the lcd
    let  config = Config::default();
    let mut i2c = I2c::new_blocking(p.I2C0, scl, sda, config);
    let peripherals = Peripherals::take().unwrap();
    let mut delay = CortexDelay::new(peripherals.SYST, 125_000_000); 
        let mut lcd = Lcd::new(&mut i2c, &mut delay)// Create an Lcd instance
        .address(LCD_ADDRESS) // Set the address
        .cursor_on(false) // Set cursor visibility
        .rows(2) // Set number of rows
        .init().unwrap(); // Initialize the LCD 

        // Set the GPIO pins for other parts:

    let mut led_pin = Output::new(p.PIN_16, gpio::Level::Low);//RED LED 
    let mut led_pin2 = Output::new(p.PIN_17, gpio::Level::Low);//GREEN LED
    let mut led_pin3 = Output::new(p.PIN_18, gpio::Level::Low);//BLUE LED
    let mut password_index = 0; // The index of the entered password
    let pir_pin = Input::new(p.PIN_3, Pull::Up);

    //The keypad initialization

    let col_pin_1 = Input::new(p.PIN_5, Pull::Down);
    let col_pin_2 = Input::new(p.PIN_4, Pull::Down);
    let col_pin_3 = Input::new(p.PIN_7, Pull::Down);
    let col_pin_4 = Input::new(p.PIN_6, Pull::Down);

    //Initialize the PWM configuration for the buzzer

    let mut buzzer_pwm_config: PwmConfig = Default::default();
    buzzer_pwm_config.top = 0xFFFF; // Set the top value 
    buzzer_pwm_config.compare_b = 50000; // Set a higher duty cycle for louder sound
    let mut buzzer = Pwm::new_output_b(p.PWM_CH0, p.PIN_1, buzzer_pwm_config.clone());// Create a PWM instance for the buzzer pin
    
    //Setting the states of the pins

    led_pin.set_low();
    led_pin2.set_low();
    led_pin3.set_low();
    buzzer.set_config(&PwmConfig::default());

    // Wait for the LCD to initialize

    Timer::after(Duration::from_millis(500)).await;

    // Write initial messages
    
    lcd.clear().unwrap();
    lcd.set_cursor(0, 0).unwrap();
    lcd.write_str("Burglar").unwrap();
    lcd.set_cursor(1, 8).unwrap();
    lcd.write_str("alarm").unwrap();
    Timer::after(Duration::from_secs(2)).await;
    lcd.clear().unwrap();
   
    lcd.set_cursor(0, 0).unwrap();
    lcd.write_str("Calibrating PIR").unwrap();
    lcd.set_cursor(1, 0).unwrap();
    lcd.write_str("Please wait...").unwrap();
    Timer::after(Duration::from_millis(1500)).await;

    for i in (1..=5).rev() {
        lcd.clear().unwrap();
        lcd.set_cursor(0, 4).unwrap();
        ufmt::uwrite!(&mut lcd, "{}", i).unwrap(); // Write the current countdown number
        lcd.write_str(" seconds").unwrap();
    
            //Visual feedback for the calibration process
        led_pin.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin.set_low();
            led_pin2.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin2.set_low();
            led_pin3.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin3.set_low();
        Timer::after(Duration::from_secs(1)).await; 
    }

    // Wait for the PIR sensor to calibrate

    lcd.clear().unwrap();
    lcd.set_cursor(0, 0).unwrap();
    lcd.write_str("Initializing").unwrap();
    lcd.set_cursor(1, 5).unwrap();
    lcd.write_str("software...").unwrap();
    Timer::after(Duration::from_millis(1500)).await;

    for i in (1..=5).rev() {
        lcd.clear().unwrap();
        lcd.set_cursor(0, 4).unwrap();
        ufmt::uwrite!(&mut lcd, "{}", i).unwrap(); // Write the current countdown number
        lcd.write_str(" seconds").unwrap();
    
        // Visual feedback for the calibration process
        led_pin.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin.set_low();
            led_pin2.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin2.set_low();
            led_pin3.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin3.set_low();
    
        Timer::after(Duration::from_secs(1)).await; 
    }

    // Display the message for the armed alarm on the LCD  

    lcd.clear().unwrap();
    lcd.set_cursor(0, 0).unwrap();
    lcd.write_str("Alarm").unwrap();
    lcd.set_cursor(1, 7).unwrap();
    lcd.write_str("armed!").unwrap();
    led_pin3.set_high(); // Turn on the blue LED
    Timer::after(Duration::from_millis(1500)).await;
    lcd.clear().unwrap();
    lcd.set_cursor(0, 0).unwrap();
    lcd.write_str("Enter    custom").unwrap();
    lcd.set_cursor(1, 0).unwrap();
    lcd.write_str("password :").unwrap();
    Timer::after(Duration::from_millis(200)).await;
    led_pin3.set_low(); // Turn off the blue LED
    // Wait for the user to enter the custom password

    for i in 0..4 {
        while !col_pin_1.is_high() && !col_pin_2.is_high() && !col_pin_3.is_high() && !col_pin_4.is_high() {
            Timer::after(Duration::from_millis(100)).await; // Poll every 100ms
        }
        if col_pin_1.is_high() {
            correct_password[i] = KEYPAD_MAPPING[0];
        } else if col_pin_2.is_high() {
            correct_password[i] = KEYPAD_MAPPING[1];
        } else if col_pin_3.is_high() {
            correct_password[i] = KEYPAD_MAPPING[2];
        } else if col_pin_4.is_high() {
            correct_password[i] = KEYPAD_MAPPING[3];
        }
        lcd.set_cursor(1, 11 + i as u8).unwrap();
        lcd.write_char(correct_password[i]).unwrap(); // Display digit for each entered digit

         // Activate the buzzer for a short duration after each digit is entered

         buzzer.set_config(&buzzer_pwm_config); // Turn on the buzzer
         Timer::after(Duration::from_millis(100)).await; // Wait for 100ms
         buzzer.set_config(&PwmConfig::default()); // Turn off the buzzer
    }

    // Wait for the user to enter the custom password
        led_pin3.set_high(); // Turn on the blue LED
    Timer::after(Duration::from_millis(1500)).await;
    lcd.clear().unwrap();
    lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Waiting  for").unwrap();
        lcd.set_cursor(1, 7).unwrap();
        lcd.write_str("motion...").unwrap();
        Timer::after(Duration::from_secs(2)).await;
        
        // Main loop

    loop {

        // Wait for motion

        while !pir_pin.is_high() {
            Timer::after(Duration::from_millis(100)).await; // Poll every 100ms
        }
        led_pin3.set_low(); // Turn off the blue LED
        buzzer.set_config(&buzzer_pwm_config); // Activate the buzzer using PWM
        
        //Some visual game for the leds

        for _ in 0..3 {
            led_pin.set_high(); 
            Timer::after(Duration::from_millis(200)).await;
            led_pin.set_low(); 
            led_pin2.set_high(); 
            Timer::after(Duration::from_millis(200)).await;
            led_pin2.set_low(); 
            led_pin3.set_high(); 
            Timer::after(Duration::from_millis(200)).await;
            led_pin3.set_low(); 
        }
        led_pin.set_high();
        led_pin2.set_low();
        led_pin3.set_low();

        // Display the motion alert message on the LCD

        lcd.clear().unwrap();
        lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Motion alert!!!").unwrap();
        Timer::after(Duration::from_secs(2)).await;
        lcd.clear().unwrap();
        lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Enter  password:").unwrap();

        // Wait for the user to enter the password

        while password_index < correct_password.len() {

            // Check each column pin

            for i in 0..4 {
                match i {
                    0 => {
                        if col_pin_1.is_high(){
                            entered_password[password_index] = KEYPAD_MAPPING[i];
                            password_index += 1;
                            lcd.set_cursor(1, password_index as u8).unwrap();
                            lcd.write_char(KEYPAD_MAPPING[i]).unwrap();
                                break;
                            
                        }
                    },
                    1 => {
                        if col_pin_2.is_high() {
                            entered_password[password_index] = KEYPAD_MAPPING[i];
                            password_index += 1;
                            lcd.set_cursor(1, password_index as u8).unwrap();
                            lcd.write_char(KEYPAD_MAPPING[i]).unwrap();
                            break;
                        }
                    },
                    2 => {
                        if col_pin_3.is_high() {
                            entered_password[password_index] = KEYPAD_MAPPING[i];
                            password_index += 1;
                            lcd.set_cursor(1, password_index as u8).unwrap();
                            lcd.write_char(KEYPAD_MAPPING[i]).unwrap();
                            break;
                        }
                    },
                    3 => {
                        if col_pin_4.is_high() {
                            entered_password[password_index] = KEYPAD_MAPPING[i];
                            password_index += 1;
                            lcd.set_cursor(1, password_index as u8).unwrap();
                            lcd.write_char(KEYPAD_MAPPING[i]).unwrap();
                            break;
                        }
                    },
                    _ => unreachable!(),
                }
            }
            Timer::after(Duration::from_millis(100)).await; // Poll every 100ms

            // Check if the entered password is incorrect

            if password_index == correct_password.len() {
                if entered_password != correct_password {
                    password_index = 0;
                    entered_password = ['0', '0', '0', '0'];
                    incorrect_attempts += 1; // Increment the incorrect attempts counter

                    // Check if the maximum number of attempts has been reached
                    if incorrect_attempts >= MAX_PASSWORD_ATTEMPTS {
                        for _ in 0..3 {

                            // Visual feedback for too many wrong attempts
                            led_pin.set_high(); 
                            Timer::after(Duration::from_millis(200)).await;
                            led_pin.set_low(); 
                            led_pin2.set_high(); 
                            Timer::after(Duration::from_millis(200)).await;
                            led_pin2.set_low();             
                            led_pin3.set_high();              
                            Timer::after(Duration::from_millis(200)).await;              
                            led_pin3.set_low(); 
                        }
                        led_pin.set_high();// Turn on the red LED
                        led_pin2.set_low();
                        led_pin3.set_low();

                        // Display the message for too many wrong attempts on the LCD

                        Timer::after(Duration::from_millis(1000)).await;
                        lcd.clear().unwrap();
                        lcd.write_str("Too many wrong").unwrap();
                            lcd.set_cursor(1, 0).unwrap();
                            lcd.write_str("attempts.Wait.. ").unwrap();
                            Timer::after(Duration::from_millis(1700)).await;
                            led_pin.set_low();
                        // If the maximum number of attempts has been reached, lock the system for a certain period of time

                        for i in (1..=5).rev() {
                            lcd.clear().unwrap();
                            lcd.set_cursor(0, 4).unwrap();
                            ufmt::uwrite!(&mut lcd, "{}", i).unwrap(); // Write the current countdown number
                            lcd.write_str(" seconds").unwrap();
                            Timer::after(Duration::from_secs(1)).await; // Wait for 1 second
                            // Visual feedback for the countdown
                                led_pin.set_high();
                                Timer::after(Duration::from_millis(200)).await;
                                led_pin.set_low();
                                led_pin2.set_high();
                                Timer::after(Duration::from_millis(200)).await;
                                led_pin2.set_low();
                                led_pin3.set_high();
                                Timer::after(Duration::from_millis(200)).await;
                                led_pin3.set_low();
                            
                        }
                        led_pin.set_high();
                        lcd.clear().unwrap();
                        incorrect_attempts = 0; // Reset the incorrect attempts counter
                        lcd.set_cursor(0, 0).unwrap();
                        lcd.write_str("Enter password:").unwrap();
                    }else {
                        Timer::after(Duration::from_millis(1000)).await;
                        for _ in 0..3 {

                            // Visual feedback for incorrect password

                            led_pin.set_high();                
                            Timer::after(Duration::from_millis(200)).await;               
                            led_pin.set_low();              
                            led_pin2.set_high();              
                            Timer::after(Duration::from_millis(200)).await;             
                            led_pin2.set_low();            
                            led_pin3.set_high();             
                            Timer::after(Duration::from_millis(200)).await;               
                            led_pin3.set_low();                
                        }
                        led_pin.set_high();// Turn on the red LED
                        led_pin2.set_low();
                        led_pin3.set_low();

                        // Display the message for incorrect password on the LCD

                        lcd.clear().unwrap();
                        lcd.set_cursor(0, 0).unwrap();
                        lcd.write_str("Password ").unwrap();
                        lcd.set_cursor(1, 6).unwrap();
                        lcd.write_str("incorrect").unwrap();
                        Timer::after(Duration::from_secs(1)).await;
                        lcd.clear().unwrap();
                        lcd.set_cursor(0, 0).unwrap();
                        lcd.write_str("Enter password:").unwrap();
                    }
                }
            }
        }

        // Check if the entered password is correct

        if entered_password == correct_password {

        // Visual feedback for correct password

        for _ in 0..3 {
            led_pin.set_high(); 
            Timer::after(Duration::from_millis(200)).await;
            led_pin.set_low(); 
            led_pin2.set_high();
            Timer::after(Duration::from_millis(200)).await;
            led_pin2.set_low();
            led_pin3.set_high(); 
            Timer::after(Duration::from_millis(200)).await;
            led_pin3.set_low(); 
        }
        led_pin2.set_high();// Turn on the green LED
        led_pin.set_low();
        led_pin3.set_low();
        
        

        // Display the message for correct password on the LCD
       
        lcd.clear().unwrap();
        lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Password correct").unwrap();
        // Play the alarm sound
        let music: [(u32, u64); 9] = [
            (44000, 500),
            (44000, 500),    
            (44000, 500),
            (34900, 350),
            (52300, 150),  
            (44000, 500),
            (34900, 350),
            (52300, 150),
            (44000, 650),
          
           
    ];

    for &(frequency, duration) in music.iter() {
        if frequency == 0 {
            buzzer.set_config(&PwmConfig::default()); // Turn off the buzzer
        } else {
            let mut buzzer_pwm_config = PwmConfig::default();
            buzzer_pwm_config.top = frequency as u16; // Set the frequency
            buzzer_pwm_config.compare_b = buzzer_pwm_config.top / 2; // Set the duty cycle to 50%
            buzzer.set_config(&buzzer_pwm_config); // Turn on the buzzer
        }
        Timer::after(Duration::from_millis(duration as u64)).await; // Convert duration to u64
    }
    // Turn off the buzzer

    buzzer.set_config(&PwmConfig::default()); 
        
        lcd.clear().unwrap();
        lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Alarm").unwrap();
        lcd.set_cursor(1, 6).unwrap();
        Timer::after(Duration::from_millis(10)).await;
        lcd.write_str("disarmed  ").unwrap();

        // Reset the password index for the next detection
        
        password_index = 0;

        // Wait for 3 seconds before checking for motion again

        Timer::after(Duration::from_secs(3)).await;
        led_pin2.set_low();
        lcd.clear().unwrap();
        lcd.set_cursor(0, 0).unwrap();
        lcd.write_str("Waiting  for").unwrap();
        lcd.set_cursor(1, 7).unwrap();
        lcd.write_str("motion...").unwrap();
        Timer::after(Duration::from_millis(200)).await;
        led_pin3.set_high(); // Turn on the blue LED
    } 
}
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
   


