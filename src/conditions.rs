use rand::{distributions::Uniform, prelude::ThreadRng};
use rand::prelude::{Distribution};
use std::convert::TryFrom;

pub struct Conditions
{
    pub lag: u128,
    pub jitter: i128,
    pub dup_chance: f32,
    pub unorder_chance: f32,
    pub loss_chance: f32,

    jitter_dist : Uniform::<i128>,
    roll_dist : Uniform<f32>,
}

impl Conditions
{
    pub fn new(lag: u128, jitter: i128, dup_chance : f32, unorder_chance : f32, loss_chance : f32) -> Conditions
    {
        Conditions{
            lag,
            jitter,
            dup_chance,
            unorder_chance, 
            loss_chance,

            jitter_dist: Uniform::from(-jitter..(jitter + 1)),
            roll_dist: Uniform::from(0f32..100f32),
        }
    }

    pub fn gen_sent_date(self, rng : &mut ThreadRng) -> u128
    {
        let now = crate::now_nanos();
        let sent_date = now + self.gen_delay(rng);
        return sent_date;
    }

    pub fn gen_delay(self, rng : &mut ThreadRng) -> u128
    {
        let network_delay: u128 = crate::millis_to_nanos(self.lag);
        let jitter = crate::i128_millis_to_nanos(self.jitter_dist.sample(rng));
        let delay = u128::try_from(std::cmp::max(network_delay as i128 + jitter, 0)).expect("Conversion error");
        return delay;
    }

    pub fn arrived(self, rng: &mut ThreadRng) -> bool
    {
        return self.roll_dist.sample(rng) > self.loss_chance;
    }

    pub fn duplicated(self, rng: &mut ThreadRng) -> bool
    {
        return self.roll_dist.sample(rng) < self.dup_chance;
    }

    pub fn unordered(self, rng: &mut ThreadRng) -> bool
    {
        return self.roll_dist.sample(rng) < self.unorder_chance;
    }

    fn millis_to_nanos(millis : u128) -> u128{
        millis * 1_000_000
    }
    
    fn i128_millis_to_nanos(millis : i128) -> i128
    {
        millis * 1_000_000
    }

    fn now_nanos() -> u128{
        let time = std::time::SystemTime::now();
        let now = time.duration_since(std::time::UNIX_EPOCH).expect("Time is wrong");
        now.as_nanos()
    }
}