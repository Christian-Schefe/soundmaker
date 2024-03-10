use fundsp::prelude::*;

use crate::node::FixedAudioNode;

#[inline]
fn optimal4x44<T: Float>(a0: T, a1: T, a2: T, a3: T, x: T) -> T {
    // Interpolator sourced from:
    // Niemitalo, Olli, Polynomial Interpolators for High-Quality Resampling of Oversampled Audio, 2001.
    let z = x - T::from_f64(0.5);
    let even1 = a2 + a1;
    let odd1 = a2 - a1;
    let even2 = a3 + a0;
    let odd2 = a3 - a0;
    let c0 = even1 * T::from_f64(0.4656725512077848) + even2 * T::from_f64(0.03432729708429672);
    let c1 = odd1 * T::from_f64(0.5374383075356016) + odd2 * T::from_f64(0.1542946255730746);
    let c2 = even1 * T::from_f64(-0.25194210134021744) + even2 * T::from_f64(0.2519474493593906);
    let c3 = odd1 * T::from_f64(-0.46896069955075126) + odd2 * T::from_f64(0.15578800670302476);
    let c4 = even1 * T::from_f64(0.00986988334359864) + even2 * -T::from_f64(0.00989340017126506);
    (((c4 * z + c3) * z + c2) * z + c1) * z + c0
}

pub struct Wavetable {
    table: Vec<(f32, Vec<f32>)>,
}

impl Wavetable {
    pub fn new<P, A>(
        min_pitch: f64,
        max_pitch: f64,
        tables_per_octave: f64,
        phase: &P,
        amplitude: &A,
    ) -> Wavetable
    where
        P: Fn(u32) -> f64,
        A: Fn(f64, u32) -> f64,
    {
        let mut table: Vec<(f32, Vec<f32>)> = vec![];
        let mut p = min_pitch;
        let p_factor = pow(2.0, 1.0 / tables_per_octave);
        let mut max_amplitude = 0.0;

        while p <= max_pitch {
            let wave = make_wave(p, phase, amplitude);
            max_amplitude = wave.iter().fold(max_amplitude, |acc, &x| max(acc, abs(x)));
            //total_size += wave.len();
            table.push((p as f32, wave));
            p *= p_factor;
        }
        if max_amplitude > 0.0 {
            let z = 1.0 / max_amplitude;
            table.iter_mut().for_each(|t| {
                t.1.iter_mut().for_each(|x| {
                    *x *= z;
                })
            });
        }

        Wavetable { table }
    }

    #[inline]
    pub fn at(&self, i: usize, phase: f32) -> f32 {
        let table: &Vec<f32> = &self.table[i].1;
        let p = table.len() as f32 * phase;
        let i1 = unsafe { f32::to_int_unchecked::<usize>(p) };
        let w = p - i1 as f32;
        let mask = table.len() - 1;
        let i0 = i1.wrapping_sub(1) & mask;
        let i1 = i1 & mask;
        let i2 = (i1 + 1) & mask;
        let i3 = (i1 + 2) & mask;
        optimal4x44(table[i0], table[i1], table[i2], table[i3], w)
    }

    #[inline]
    pub fn read(&self, table_hint: usize, frequency: f32, phase: f32) -> (f32, usize) {
        let table =
            if frequency >= self.table[table_hint].0 && frequency <= self.table[table_hint + 1].0 {
                table_hint
            } else {
                let mut i0 = 0;
                let mut i1 = self.table.len() - 3;
                while i0 < i1 {
                    let i = (i0 + i1) >> 1;
                    if self.table[i].0 > frequency {
                        i1 = i;
                    } else if self.table[i + 1].0 > frequency {
                        i0 = i;
                        break;
                    } else {
                        i0 = i + 1;
                    }
                }
                i0
            };
        let w = delerp(self.table[table].0, self.table[table + 1].0, frequency);
        (
            (1.0 - w) * self.at(table + 1, phase) + w * self.at(table + 2, phase),
            table,
        )
    }
}

pub struct WavetableSynth {
    table: Wavetable,
    phase: f32,
    initial_phase: f32,
    table_hint: usize,
    sample_rate: f32,
}

impl WavetableSynth {
    pub fn new(table: Wavetable) -> Self {
        Self {
            table,
            phase: 0.0,
            initial_phase: 0.0,
            table_hint: 0,
            sample_rate: DEFAULT_SR as f32,
        }
    }
}

impl FixedAudioNode for WavetableSynth {
    type Inputs = U1;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let frequency = input[0] as f32;
        let delta = frequency / self.sample_rate;
        self.phase += delta;
        self.phase -= self.phase.floor();
        let (val, hint) = self
            .table
            .read(self.table_hint, frequency.abs(), self.phase);
        self.table_hint = hint;
        output[0] = val as f64;
    }

    fn reset(&mut self) {
        self.phase = self.initial_phase;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate as f32;
    }
}
