//! ARQ-129 (chain), ARQ-123 and ARQ-111 for `ProcessorChain`.

use gafira_audio::{
    AudioBlock, AudioProcessor, Gain, PARAM_VOLUME, Pan, ParamSnapshot, ProcessorChain,
};
use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;

static OP: Descriptor = Descriptor::new("test.op", MessageKey("test.op"), 1);

/// `x -> x * 2` or `x -> x + 1`, with a declared latency.
struct Op {
    double: bool,
    latency: u32,
    tail: u32,
}

impl Describe for Op {
    fn descriptor(&self) -> &Descriptor {
        &OP
    }
}

impl AudioProcessor for Op {
    fn prepare(&mut self, _: u32, _: usize, _: u16) {}
    fn process(&mut self, block: &mut AudioBlock<'_>, _: &ParamSnapshot) {
        for c in block.channels.iter_mut() {
            for x in c.iter_mut() {
                *x = if self.double { *x * 2.0 } else { *x + 1.0 };
            }
        }
    }
    fn latency(&self) -> u32 {
        self.latency
    }
    fn tail(&self) -> u32 {
        self.tail
    }
    fn reset(&mut self) {}
}

fn op(double: bool, latency: u32) -> Box<dyn AudioProcessor> {
    Box::new(Op {
        double,
        latency,
        tail: 0,
    })
}

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

#[test]
fn arq_129_the_chain_applies_its_processors_in_order() {
    let mut forward = ProcessorChain::new();
    forward.push(op(true, 0));
    forward.push(op(false, 0));
    let mut backward = ProcessorChain::new();
    backward.push(op(false, 0));
    backward.push(op(true, 0));
    for (chain, want) in [(&mut forward, 3.0), (&mut backward, 4.0)] {
        let mut b = vec![vec![1.0_f32; 2]];
        run(chain, &mut b, &ParamSnapshot::new());
        assert_eq!(b[0], vec![want; 2]);
    }
}

#[test]
fn arq_129_the_latency_of_a_chain_is_the_sum() {
    let mut chain = ProcessorChain::new();
    assert_eq!(chain.latency(), 0);
    chain.push(op(true, 3));
    chain.push(op(false, 5));
    assert_eq!(
        (chain.latency(), chain.len(), chain.is_empty()),
        (8, 2, false)
    );
}

#[test]
fn arq_129_a_latency_sum_that_overflows_saturates_without_panic() {
    let mut chain = ProcessorChain::new();
    chain.push(op(true, u32::MAX));
    chain.push(op(false, 5));
    assert_eq!(chain.latency(), u32::MAX);
}

#[test]
fn arq_123_an_empty_chain_cleans_the_block_and_keeps_normal_samples() {
    // Prepared and not prepared alike: an empty chain is still a processor.
    for prepared in [true, false] {
        let mut chain = ProcessorChain::new();
        assert!(chain.is_empty());
        if prepared {
            chain.prepare(48_000, 64, 1);
        }
        let tiny = f32::MIN_POSITIVE * 0.5;
        let mut b = vec![vec![
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            tiny,
            -tiny,
            0.5,
            -0.25,
        ]];
        run(&mut chain, &mut b, &ParamSnapshot::new());
        assert_eq!(
            b[0],
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.5, -0.25],
            "{prepared}"
        );
    }
}

#[test]
fn arq_129_the_tail_of_a_chain_is_the_sum_and_saturates() {
    let with_tail = |tail| -> Box<dyn AudioProcessor> {
        Box::new(Op {
            double: true,
            latency: 0,
            tail,
        })
    };
    let mut chain = ProcessorChain::new();
    assert_eq!(chain.tail(), 0);
    chain.push(with_tail(7));
    chain.push(with_tail(11));
    assert_eq!(chain.tail(), 18);
    chain.push(with_tail(u32::MAX));
    assert_eq!(chain.tail(), u32::MAX);
}

#[test]
fn arq_129_prepare_and_reset_reach_every_processor() {
    let mut chain = ProcessorChain::new();
    chain.push(Box::new(Gain::new()));
    chain.push(Box::new(Pan::new()));
    let mut p = ParamSnapshot::new();
    p.set(PARAM_VOLUME, 0.0);

    // Not prepared: nothing is processed, the block is only cleaned.
    let mut b = vec![vec![f32::NAN; 4], vec![1.0; 4]];
    run(&mut chain, &mut b, &p);
    assert!(b[0][0] == 0.0 && b[1][0] == 1.0);

    // Prepared: a ramp runs in the gain; reset ends it at its target.
    chain.prepare(48_000, 64, 2);
    let mut first = vec![vec![1.0; 8], vec![1.0; 8]];
    run(&mut chain, &mut first, &ParamSnapshot::new());
    let mut b = vec![vec![1.0; 8], vec![1.0; 8]];
    run(&mut chain, &mut b, &p);
    assert!(b[0][7] > 0.0 && b[1][7] > 0.0, "ramping");
    chain.reset();
    let mut b = vec![vec![1.0; 8], vec![1.0; 8]];
    run(&mut chain, &mut b, &p);
    assert_eq!(b, vec![vec![0.0; 8], vec![0.0; 8]]);
}

#[test]
fn arq_123_a_chain_is_total_on_hostile_input() {
    let mut chain = ProcessorChain::new();
    chain.push(Box::new(Gain::new()));
    chain.push(Box::new(Pan::new()));
    chain.prepare(48_000, 16, 2);
    let mut p = ParamSnapshot::new();
    p.set(PARAM_VOLUME, 200.0);
    let row = vec![
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        -f32::MAX,
        1e-40,
    ];
    let mut b = vec![row.clone(), row];
    run(&mut chain, &mut b, &p);
    assert!(
        b.iter()
            .flatten()
            .all(|x| x.is_finite() && !x.is_subnormal())
    );
}
