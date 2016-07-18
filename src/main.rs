//! An example of using dsp-chain's `Graph` type to create a simple Synthesiser
//! with 3 sine wave
//! oscillators.

extern crate dsp;
extern crate portaudio;
extern crate rand;

use std::io::{Stdin, Read};

use dsp::{Graph, Node, Frame, FromSample, Sample, Walker};
use dsp::sample::ToFrameSliceMut;
use portaudio as pa;

use std::sync::mpsc::{channel, Sender};
use std::thread;

use self::Command::*;

/// SoundStream is currently generic over i8, i32 and f32. Feel free to change
/// it!
type Output = f32;

type Phase = f64;
type Frequency = f64;
type Volume = f32;

const CHANNELS: usize = 2;
const FRAMES: u32 = 64;
const SAMPLE_HZ: f64 = 44_100.0;


enum Command {
    Play(Frequency, Frequency),
    Stop,
}


fn to_frequencies(character : char) -> Option<(Frequency, Frequency)> {
    match character {
        '1' => Some((1209.0, 697.0)),
        '2' => Some((1336.0, 697.0)),
        '3' => Some((1477.0, 697.0)),
        'A' => Some((1633.0, 697.0)),
        '4' => Some((1209.0, 770.0)),
        '5' => Some((1336.0, 770.0)),
        '6' => Some((1477.0, 770.0)),
        'B' => Some((1633.0, 770.0)),
        '7' => Some((1209.0, 852.0)),
        '8' => Some((1336.0, 852.0)),
        '9' => Some((1477.0, 852.0)),
        'C' => Some((1633.0, 852.0)),
        '*' => Some((1209.0, 941.0)),
        '0' => Some((1336.0, 941.0)),
        '#' => Some((1477.0, 941.0)),
        'D' => Some((1633.0, 941.0)),
        _ => None,
    }
}


fn play(channel : &Sender<Command>, character : char) {
    if let Some((a, b)) = to_frequencies(character) {
        channel.send(Play(a, b)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        channel.send(Stop).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}


fn run() -> Result<(), pa::Error> {

    // Construct our dsp graph.
    let mut graph = Graph::new();

    // Construct our fancy Synth and add it to the graph!
    let synth = graph.add_node(DspNode::Synth);

    // Connect a few oscillators to the synth.
    let (_, oscillator_a) = graph.add_input(DspNode::Oscillator(0.0, 0.0, 0.0), synth);
    let (_, oscillator_b) = graph.add_input(DspNode::Oscillator(0.0, 0.0, 0.0), synth);

    // Set the synth as the master node for the graph.
    graph.set_master(Some(synth));

    let (tx, rx) = channel();

    // The callback we'll use to pass to the Stream. It will request audio from
    // our dsp_graph.
    let callback = move |pa::OutputStreamCallbackArgs { buffer, time, .. }| {
        let buffer: &mut [[Output; CHANNELS]] = buffer.to_frame_slice_mut().unwrap();

        dsp::slice::equilibrium(buffer);
        graph.audio_requested(buffer, SAMPLE_HZ);

        match rx.try_recv() {
            Ok(Play(freq_a, freq_b)) => {
                if let DspNode::Oscillator(_, ref mut pitch, ref mut volume) = graph[oscillator_a] {
                    *pitch = freq_a;
                    *volume = 0.2;
                }

                if let DspNode::Oscillator(_, ref mut pitch, ref mut volume) = graph[oscillator_b] {
                    *pitch = freq_b;
                    *volume = 0.2;
                }

                pa::Continue
            }

            Ok(Stop) => {
                if let DspNode::Oscillator(_, ref mut pitch, ref mut volume) = graph[oscillator_a] {
                    *volume = 0.0;
                }

                if let DspNode::Oscillator(_, ref mut pitch, ref mut volume) = graph[oscillator_b] {
                    *volume = 0.0;
                }

                pa::Continue
            }

            Err(std::sync::mpsc::TryRecvError::Empty) => {
                pa::Continue
            }

            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                pa::Complete
            }
        }
    };

    // Construct PortAudio and the stream.
    let pa = try!(pa::PortAudio::new());
    let settings = try!(pa.default_output_stream_settings::<Output>(CHANNELS as i32, SAMPLE_HZ, FRAMES));
    let mut stream = try!(pa.open_non_blocking_stream(settings, callback));
    try!(stream.start());

    // Read
    for character in std::io::stdin().bytes() {
        play(&tx, character.unwrap() as char);
    }

    //for character in "123A456B789C*0#D".chars() {
    //    play(&tx, character);
    //}

    Ok(())
}

/// Our type for which we will implement the `Dsp` trait.
#[derive(Debug)]
enum DspNode {
    /// Synth will be our demonstration of a master GraphNode.
    Synth,
    /// Oscillator will be our generator type of node, meaning that we will override
    /// the way it provides audio via its `audio_requested` method.
    Oscillator(Phase, Frequency, Volume),
}

impl Node<[Output; CHANNELS]> for DspNode {
    /// Here we'll override the audio_requested method and generate a sine wave.
    fn audio_requested(&mut self, buffer: &mut [[Output; CHANNELS]], sample_hz: f64) {
        match *self {
            DspNode::Synth => (),
            DspNode::Oscillator(ref mut phase, frequency, volume) => {
                dsp::slice::map_in_place(buffer, |_| {
                    let val = sine_wave(*phase, volume);
                    *phase += frequency / sample_hz;
                    Frame::from_fn(|_| val)
                });
            },
        }
    }
}

/// Return a sine wave for the given phase.
fn sine_wave<S: Sample>(phase: Phase, volume: Volume) -> S
    where S: Sample + FromSample<f32>,
{
    use std::f64::consts::PI;
    ((phase * PI * 2.0).sin() as f32 * volume).to_sample::<S>()
}

fn main() {
    run().unwrap()
}
