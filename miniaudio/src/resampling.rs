use crate::base::{Error, Format};
use crate::frames::{Frame, Frames, Sample};
use miniaudio_sys as sys;
use std::marker::PhantomData;

/// The choice of resampling algorithm depends on your situation and requirements.
/// The linear resampler is the most efficient and has the least amount of latency,
/// but at the expense of poorer quality. The Speex resampler is higher quality,
/// but slower with more latency. It also performs several heap allocations internally
/// for memory management.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResampleAlgorithm {
    /// Fastest, lowest quality, optional low-pass filtering. Default.
    Linear = sys::ma_resample_algorithm_linear as _,
    Speex = sys::ma_resample_algorithm_speex as _,
}
impl_from_c!(ResampleAlgorithm, sys::ma_resample_algorithm);

impl Default for ResampleAlgorithm {
    fn default() -> ResampleAlgorithm {
        ResampleAlgorithm::Linear
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub struct LinearResamplerConfig<S: Sample, F: Frame>(
    sys::ma_linear_resampler_config,
    PhantomData<S>,
    PhantomData<F>,
);

impl<S: Sample, F: Frame> LinearResamplerConfig<S, F> {
    #[inline]
    pub fn new(sample_rate_in: u32, sample_rate_out: u32) -> LinearResamplerConfig<S, F> {
        LinearResamplerConfig(
            unsafe {
                sys::ma_linear_resampler_config_init(
                    S::format() as _,
                    S::channels::<F>() as _,
                    sample_rate_in,
                    sample_rate_out,
                )
            },
            PhantomData,
            PhantomData,
        )
    }

    #[inline]
    pub fn format(&self) -> Format {
        Format::from_c(self.0.format)
    }

    pub fn channels(&self) -> u32 {
        self.0.channels
    }

    #[inline]
    pub fn sample_rate_in(&self) -> u32 {
        self.0.sampleRateIn
    }

    #[inline]
    pub fn set_sample_rate_in(&mut self, sample_rate: u32) {
        self.0.sampleRateIn = sample_rate;
    }

    #[inline]
    pub fn sample_rate_out(&self) -> u32 {
        self.0.sampleRateOut
    }

    #[inline]
    pub fn set_sample_rate_out(&mut self, sample_rate: u32) {
        self.0.sampleRateOut = sample_rate;
    }

    /// The low pass filter order. If this is set to 0, low pass filtering will be disabled.
    #[inline]
    pub fn lpf_order(&self) -> u32 {
        self.0.lpfOrder
    }

    /// The low pass filter order. If this is set to 0, low pass filtering will be disabled.
    #[inline]
    pub fn set_lpf_order(&mut self, lpf_order: u32) {
        self.0.lpfOrder = lpf_order;
    }

    /// 0..1. Defaults to 1. 1 = Half the sampling frequency (Nyquist Frequency),
    /// 0.5 = Quarter the sampling frequency (half Nyquest Frequency), etc.
    #[inline]
    pub fn lpf_nyquist_factor(&self) -> f64 {
        self.0.lpfNyquistFactor
    }

    /// 0..1. Defaults to 1. 1 = Half the sampling frequency (Nyquist Frequency),
    /// 0.5 = Quarter the sampling frequency (half Nyquest Frequency), etc.
    #[inline]
    pub fn set_lpf_nyquist_factor(&mut self, factor: f64) {
        self.0.lpfNyquistFactor = factor;
    }
}

#[repr(transparent)]
pub struct LinearResampler<S: Sample, F: Frame>(
    sys::ma_linear_resampler,
    PhantomData<S>,
    PhantomData<F>,
);

impl<S: Sample, F: Frame> LinearResampler<S, F> {
    #[inline]
    pub fn new(config: &LinearResamplerConfig<S, F>) -> Result<LinearResampler<S, F>, Error> {
        let mut lr = std::mem::MaybeUninit::<LinearResampler<S, F>>::uninit();
        unsafe {
            Error::from_c_result(sys::ma_linear_resampler_init(
                config as *const LinearResamplerConfig<S, F> as *const _,
                lr.as_mut_ptr() as *mut _,
            ))?;
            Ok(lr.assume_init())
        }
    }

    #[inline]
    pub fn config(&self) -> &LinearResamplerConfig<S, F> {
        unsafe { std::mem::transmute(&self.0.config) }
    }

    // FIXME this API actually allows passing null for input or output and does this:
    //
    //      You can pass in NULL for the input buffer in which case it will be treated
    //      as an infinitely large buffer of zeros. The output buffer can also be NULL,
    //      in which case the processing will be treated as seek.
    //
    // I don't have a really good way to represent this right now, so I don't support it :P.
    //
    /// Converts the given input data.
    ///
    /// Returns the number of input frames that were consumed during processing and the number of
    /// output frames that were written to the output buffer respectively.
    #[inline]
    pub fn process_pcm_frames(
        &mut self,
        output: &mut Frames<S, F>,
        input: &Frames<S, F>,
    ) -> Result<(u64, u64), Error> {
        let mut output_frames = output.count() as u64;
        let mut input_frames = input.count() as u64;

        Error::from_c_result(unsafe {
            sys::ma_linear_resampler_process_pcm_frames(
                &mut self.0,
                input.frames_ptr() as *const _,
                &mut input_frames,
                output.frames_ptr_mut() as *mut _,
                &mut output_frames,
            )
        })?;

        return Ok((input_frames, output_frames));
    }

    /// Sets the input and output sample rate.
    #[inline]
    pub fn set_rate(&mut self, sample_rate_in: u32, sample_rate_out: u32) -> Result<(), Error> {
        Error::from_c_result(unsafe {
            sys::ma_linear_resampler_set_rate(&mut self.0, sample_rate_in, sample_rate_out)
        })
    }

    /// Sets the input and output sample rate as a ratio.
    ///
    /// The ratio is in/out.
    #[inline]
    pub fn set_rate_ratio(&mut self, ratio_in_out: f32) -> Result<(), Error> {
        Error::from_c_result(unsafe {
            sys::ma_linear_resampler_set_rate_ratio(&mut self.0, ratio_in_out)
        })
    }

    /// Calculates the number of whole input frames that would need to be read from the client in
    /// order to output the specified number of output frames.
    ///
    /// The returned value does not include cached input frames. It only returns the number of
    /// extra frames that would need to be read from the input buffer in order to output the
    /// specified number of output frames.
    #[inline]
    pub fn required_input_frame_count(&self, output_frame_count: u64) -> u64 {
        unsafe {
            sys::ma_linear_resampler_get_required_input_frame_count(
                &self.0 as *const _ as *mut _,
                output_frame_count,
            )
        }
    }

    /// Calculates the number of whole output frames that would be output after fully reading and
    /// consuming the specified number of input frames.
    #[inline]
    pub fn expected_output_frame_count(&self, input_frame_count: u64) -> u64 {
        unsafe {
            sys::ma_linear_resampler_get_expected_output_frame_count(
                &self.0 as *const _ as *mut _,
                input_frame_count,
            )
        }
    }
    #[inline]

    /// Retrieves the latency introduced by the resampler in input frames.
    pub fn input_latency(&mut self) -> u64 {
        unsafe { sys::ma_linear_resampler_get_input_latency(&self.0 as *const _ as *mut _) }
    }

    /// Retrieves the latency introduced by the resampler in output frames.
    #[inline]
    pub fn output_latency(&mut self) -> u64 {
        unsafe { sys::ma_linear_resampler_get_output_latency(&self.0 as *const _ as *mut _) }
    }

    #[inline]
    pub fn in_advance_int(&self) -> u32 {
        self.0.inAdvanceInt
    }

    #[inline]
    pub fn in_advance_frac(&self) -> u32 {
        self.0.inAdvanceFrac
    }

    #[inline]
    pub fn in_time_int(&self) -> u32 {
        self.0.inTimeInt
    }

    #[inline]
    pub fn in_time_frac(&self) -> u32 {
        self.0.inTimeFrac
    }
}

impl<S: Sample, F: Frame> Drop for LinearResampler<S, F> {
    fn drop(&mut self) {
        unsafe { sys::ma_linear_resampler_uninit(self as *mut LinearResampler<S, F> as *mut _) };
    }
}

impl<S: Sample, F: Frame> Clone for LinearResampler<S, F> {
    fn clone(&self) -> Self {
        // This shouldn't fail assuming this was initialized properly to start with.
        Self::new(self.config()).expect("failed to clone linear resampler")
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub struct ResamplerConfig<S: Sample, F: Frame>(
    sys::ma_resampler_config,
    PhantomData<S>,
    PhantomData<F>,
);

impl<S: Sample, F: Frame> ResamplerConfig<S, F> {
    pub fn new(
        sample_rate_in: u32,
        sample_rate_out: u32,
        algorithm: ResampleAlgorithm,
    ) -> ResamplerConfig<S, F> {
        ResamplerConfig(
            unsafe {
                sys::ma_resampler_config_init(
                    S::format() as _,
                    S::channels::<F>() as _,
                    sample_rate_in,
                    sample_rate_out,
                    algorithm as _,
                )
            },
            PhantomData,
            PhantomData,
        )
    }

    #[inline]
    pub fn format(&self) -> Format {
        Format::from_c(self.0.format)
    }

    #[inline]
    pub fn channels(&self) -> u32 {
        self.0.channels
    }

    #[inline]
    pub fn sample_rate_in(&self) -> u32 {
        self.0.sampleRateIn
    }

    #[inline]
    pub fn set_sample_rate_in(&mut self, sample_rate: u32) {
        self.0.sampleRateIn = sample_rate;
    }

    #[inline]
    pub fn sample_rate_out(&self) -> u32 {
        self.0.sampleRateOut
    }

    #[inline]
    pub fn set_sample_rate_out(&mut self, sample_rate: u32) {
        self.0.sampleRateOut = sample_rate;
    }

    #[inline]
    pub fn algorithm(&self) -> ResampleAlgorithm {
        ResampleAlgorithm::from_c(self.0.algorithm)
    }

    #[inline]
    pub fn set_algorithm(&mut self, algorithm: ResampleAlgorithm) {
        self.0.algorithm = algorithm as _;
    }

    #[inline]
    pub fn linear_lpf_order(&self) -> u32 {
        self.0.linear.lpfOrder
    }

    #[inline]
    pub fn set_linear_lpf_order(&mut self, order: u32) {
        self.0.linear.lpfOrder = order;
    }

    #[inline]
    pub fn linear_lpf_nyquist_factor(&self) -> f64 {
        self.0.linear.lpfNyquistFactor
    }

    #[inline]
    pub fn set_linear_lpf_nyquist_factor(&mut self, factor: f64) {
        self.0.linear.lpfNyquistFactor = factor;
    }

    /// From 0 to 10. Defaults to 3.
    #[inline]
    pub fn speex_quality(&mut self) -> u32 {
        self.0.speex.quality as u32
    }

    /// From 0 to 10. Defaults to 3.
    #[inline]
    pub fn set_speex_quality(&mut self, quality: u32) {
        self.0.speex.quality = std::cmp::min(quality, 10) as i32;
    }
}

#[repr(transparent)]
pub struct Resampler<S: Sample, F: Frame>(sys::ma_resampler, PhantomData<S>, PhantomData<F>);

impl<S: Sample, F: Frame> Resampler<S, F> {
    pub fn new(config: &ResamplerConfig<S, F>) -> Result<Resampler<S, F>, Error> {
        let mut resampler = std::mem::MaybeUninit::<Resampler<S, F>>::uninit();
        unsafe {
            sys::ma_resampler_init(&config.0, resampler.as_mut_ptr() as *mut _);
            Ok(resampler.assume_init())
        }
    }

    #[inline]
    pub fn config(&self) -> &ResamplerConfig<S, F> {
        unsafe { std::mem::transmute(&self.0.config) }
    }

    // FIXME this API actually allows passing null for input or output and does this:
    //
    //      You can pass in NULL for the input buffer in which case it will be treated
    //      as an infinitely large buffer of zeros. The output buffer can also be NULL,
    //      in which case the processing will be treated as seek.
    //
    // I don't have a really good way to represent this right now, so I don't support it :P.
    //
    /// Converts the given input data.
    ///
    /// Returns the number of input frames that were consumed during processing and the number of
    /// output frames that were written to the output buffer respectively.
    #[inline]
    pub fn process_pcm_frames(
        &mut self,
        output: &mut Frames<S, F>,
        input: &Frames<S, F>,
    ) -> Result<(u64, u64), Error> {
        let mut output_frames = output.count() as u64;
        let mut input_frames = input.count() as u64;

        Error::from_c_result(unsafe {
            sys::ma_resampler_process_pcm_frames(
                &mut self.0,
                input.frames_ptr() as *const _,
                &mut input_frames,
                output.frames_ptr_mut() as *mut _,
                &mut output_frames,
            )
        })?;

        return Ok((input_frames, output_frames));
    }

    /// Sets the input and output sample rate.
    #[inline]
    pub fn set_rate(&mut self, sample_rate_in: u32, sample_rate_out: u32) -> Result<(), Error> {
        Error::from_c_result(unsafe {
            sys::ma_resampler_set_rate(&mut self.0, sample_rate_in, sample_rate_out)
        })
    }

    /// Sets the input and output sample rate as a ratio.
    ///
    /// The ratio is in/out.
    #[inline]
    pub fn set_rate_ratio(&mut self, ratio_in_out: f32) -> Result<(), Error> {
        Error::from_c_result(unsafe { sys::ma_resampler_set_rate_ratio(&mut self.0, ratio_in_out) })
    }

    /// Calculates the number of whole input frames that would need to be read from the client in
    /// order to output the specified number of output frames.
    ///
    /// The returned value does not include cached input frames. It only returns the number of
    /// extra frames that would need to be read from the input buffer in order to output the
    /// specified number of output frames.
    #[inline]
    pub fn required_input_frame_count(&self, output_frame_count: u64) -> u64 {
        unsafe {
            sys::ma_resampler_get_required_input_frame_count(
                &self.0 as *const _ as *mut _,
                output_frame_count,
            )
        }
    }

    /// Calculates the number of whole output frames that would be output after fully reading and
    /// consuming the specified number of input frames.
    #[inline]
    pub fn expected_output_frame_count(&self, input_frame_count: u64) -> u64 {
        unsafe {
            sys::ma_linear_resampler_get_expected_output_frame_count(
                &self.0 as *const _ as *mut _,
                input_frame_count,
            )
        }
    }
    #[inline]

    /// Retrieves the latency introduced by the resampler in input frames.
    pub fn input_latency(&mut self) -> u64 {
        unsafe { sys::ma_linear_resampler_get_input_latency(&self.0 as *const _ as *mut _) }
    }

    /// Retrieves the latency introduced by the resampler in output frames.
    #[inline]
    pub fn output_latency(&mut self) -> u64 {
        unsafe { sys::ma_linear_resampler_get_output_latency(&self.0 as *const _ as *mut _) }
    }
}

impl<S: Sample, F: Frame> Clone for Resampler<S, F> {
    fn clone(&self) -> Self {
        // This should not fail if the resampler was properly initialized.
        Self::new(self.config()).expect("failed to clone resampler")
    }
}

impl<S: Sample, F: Frame> Drop for Resampler<S, F> {
    fn drop(&mut self) {
        unsafe { sys::ma_resampler_uninit(&mut self.0) };
    }
}
