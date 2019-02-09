// Copyright 2018 Stefan Kroboth
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! * [More-Thuente line search](struct.MoreThuenteLineSearch.html)
//!
//! TODO: Apparently it is missing stopping criteria!
//!
//! This implementation follows the excellent MATLAB implementation of Dianne P. O'Leary at
//! http://www.cs.umd.edu/users/oleary/software/
//!
//! # Reference
//!
//! Jorge J. More and David J. Thuente. "Line search algorithms with guaranteed sufficient
//! decrease." ACM Trans. Math. Softw. 20, 3 (September 1994), 286-307.
//! DOI: https://doi.org/10.1145/192115.192132

use crate::prelude::*;
use std;

#[derive(Default, Clone)]
struct Step {
    pub x: f64,
    pub fx: f64,
    pub gx: f64,
}

impl Step {
    pub fn new(x: f64, fx: f64, gx: f64) -> Self {
        Step { x, fx, gx }
    }
}

/// The More-Thuente line search is a method to find a step length which obeys the strong Wolfe
/// conditions.
///
/// # Example
///
/// ```
/// # extern crate argmin;
/// # use argmin::prelude::*;
/// # use argmin::solver::linesearch::MoreThuenteLineSearch;
/// # use argmin::testfunctions::{sphere, sphere_derivative};
/// #
/// # #[derive(Clone)]
/// # struct MyProblem {}
/// #
/// # impl ArgminOperator for MyProblem {
/// #     type Parameters = Vec<f64>;
/// #     type OperatorOutput = f64;
/// #     type Hessian = ();
/// #
/// #     fn apply(&self, param: &Vec<f64>) -> Result<f64, Error> {
/// #         Ok(sphere(param))
/// #     }
/// #
/// #     fn gradient(&self, param: &Vec<f64>) -> Result<Vec<f64>, Error> {
/// #         Ok(sphere_derivative(param))
/// #     }
/// # }
/// #
/// # fn run() -> Result<(), Error> {
/// // Define inital parameter vector
/// let init_param: Vec<f64> = vec![1.0, 0.0];
///
/// // Problem definition
/// let operator = MyProblem {};
///
/// // Set up line search method
/// let mut solver = MoreThuenteLineSearch::new(operator);
///
/// // Set search direction
/// solver.set_search_direction(vec![-2.0, 0.0]);
///
/// // Set initial position
/// solver.set_initial_parameter(init_param);
///
/// // Calculate initial cost ...
/// solver.calc_initial_cost()?;
/// // ... or, alternatively, set cost if it is already computed
/// // solver.set_initial_cost(...);
///
/// // Calculate initial gradient ...
/// solver.calc_initial_gradient()?;
/// // .. or, alternatively, set gradient if it is already computed
/// // solver.set_initial_gradient(...);
///
/// // Set initial step length
/// solver.set_initial_alpha(1.0)?;
///
/// // Attach a logger
/// solver.add_logger(ArgminSlogLogger::term());
///
/// // Run solver
/// solver.run()?;
///
/// // Wait a second (lets the logger flush everything before printing again)
/// std::thread::sleep(std::time::Duration::from_secs(1));
///
/// // Print Result
/// println!("{:?}", solver.result());
/// #     Ok(())
/// # }
/// #
/// # fn main() {
/// #     if let Err(ref e) = run() {
/// #         println!("{} {}", e.as_fail(), e.backtrace());
/// #     }
/// # }
/// ```
///
/// # References
///
/// This implementation follows the excellent MATLAB implementation of Dianne P. O'Leary at
/// http://www.cs.umd.edu/users/oleary/software/
///
/// [0] Jorge J. More and David J. Thuente. "Line search algorithms with guaranteed sufficient
/// decrease." ACM Trans. Math. Softw. 20, 3 (September 1994), 286-307.
/// DOI: https://doi.org/10.1145/192115.192132
#[derive(ArgminSolver)]
pub struct MoreThuenteLineSearch<T, H, O>
where
    T: std::default::Default
        + Clone
        + std::fmt::Debug
        + ArgminSub<T, T>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64, T>,
    H: Clone + std::default::Default,
    O: ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = H>,
{
    /// initial parameter vector (builder)
    init_param_b: Option<T>,
    /// initial cost (builder)
    finit_b: Option<f64>,
    /// initial gradient (builder)
    init_grad_b: Option<T>,
    /// Search direction (builder)
    search_direction_b: Option<T>,
    /// initial parameter vector
    init_param: T,
    /// initial cost
    finit: f64,
    /// initial gradient
    init_grad: T,
    /// Search direction
    search_direction: T,
    /// Search direction in 1D
    dginit: f64,
    /// dgtest
    dgtest: f64,
    /// c1
    ftol: f64,
    /// c2
    gtol: f64,
    /// xtrapf?
    xtrapf: f64,
    /// width of interval
    width: f64,
    /// width of what?
    width1: f64,
    /// xtol
    xtol: f64,
    /// alpha
    alpha: f64,
    /// stpmin
    stpmin: f64,
    /// stpmax
    stpmax: f64,
    /// current step
    stp: Step,
    /// stx
    stx: Step,
    /// sty
    sty: Step,
    /// f
    f: f64,
    /// bracketed
    brackt: bool,
    /// stage1
    stage1: bool,
    /// infoc
    infoc: usize,
    /// base
    base: ArgminBase<T, H, O>,
}

impl<T, H, O> MoreThuenteLineSearch<T, H, O>
where
    T: std::default::Default
        + Clone
        + std::fmt::Debug
        + ArgminSub<T, T>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64, T>,
    H: Clone + std::default::Default,
    MoreThuenteLineSearch<T, H, O>: ArgminSolver<Parameters = T, OperatorOutput = f64>,
    O: ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = H>,
{
    /// Constructor
    ///
    /// Parameters:
    ///
    /// `operator`: operator
    pub fn new(operator: O) -> Self {
        MoreThuenteLineSearch {
            init_param_b: None,
            finit_b: None,
            init_grad_b: None,
            search_direction_b: None,
            init_param: T::default(),
            finit: std::f64::INFINITY,
            init_grad: T::default(),
            search_direction: T::default(),
            dginit: 0.0,
            dgtest: 0.0,
            ftol: 1e-4,
            gtol: 0.9,
            xtrapf: 4.0,
            width: std::f64::NAN,
            width1: std::f64::NAN,
            xtol: 1e-10,
            alpha: 1.0,
            stpmin: std::f64::EPSILON.sqrt(),
            stpmax: std::f64::INFINITY,
            stp: Step::default(),
            stx: Step::default(),
            sty: Step::default(),
            f: std::f64::NAN,
            brackt: false,
            stage1: true,
            infoc: 1,
            base: ArgminBase::new(operator, T::default()),
        }
    }

    /// set current gradient value
    pub fn set_cur_grad(&mut self, grad: T) -> &mut Self {
        self.base.set_cur_grad(grad);
        self
    }

    /// Set c1 and c2 where 0 < c1 < c2 < 1.
    pub fn set_c(&mut self, c1: f64, c2: f64) -> Result<&mut Self, Error> {
        if c1 <= 0.0 || c1 >= c2 {
            return Err(ArgminError::InvalidParameter {
                text: "MoreThuenteLineSearch: Parameter c1 must be in (0, c2).".to_string(),
            }
            .into());
        }
        if c2 <= c1 || c2 >= 1.0 {
            return Err(ArgminError::InvalidParameter {
                text: "MoreThuenteLineSearch: Parameter c2 must be in (c1, 1).".to_string(),
            }
            .into());
        }
        self.ftol = c1;
        self.gtol = c2;
        Ok(self)
    }

    /// set alpha limits
    pub fn set_alpha_min_max(
        &mut self,
        alpha_min: f64,
        alpha_max: f64,
    ) -> Result<&mut Self, Error> {
        if alpha_min < 0.0 {
            return Err(ArgminError::InvalidParameter {
                text: "MoreThuenteLineSearch: alpha_min must be >= 0.0.".to_string(),
            }
            .into());
        }
        if alpha_max <= alpha_min {
            return Err(ArgminError::InvalidParameter {
                text: "MoreThuenteLineSearch: alpha_min must be smaller than alpha_max."
                    .to_string(),
            }
            .into());
        }
        self.stpmin = alpha_min;
        self.stpmax = alpha_max;
        Ok(self)
    }
}

impl<T, H, O> ArgminLineSearch for MoreThuenteLineSearch<T, H, O>
where
    T: std::default::Default
        + Clone
        + std::fmt::Debug
        + ArgminSub<T, T>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64, T>,
    H: Clone + std::default::Default,
    O: ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = H>,
{
    /// Set search direction
    fn set_search_direction(&mut self, search_direction: T) {
        self.search_direction_b = Some(search_direction);
    }

    /// Set initial parameter
    fn set_initial_parameter(&mut self, param: T) {
        self.init_param_b = Some(param.clone());
        self.set_cur_param(param);
    }

    /// Set initial cost function value
    fn set_initial_cost(&mut self, init_cost: f64) {
        self.finit_b = Some(init_cost);
    }

    /// Set initial gradient
    fn set_initial_gradient(&mut self, init_grad: T) {
        self.init_grad_b = Some(init_grad);
    }

    /// Calculate initial cost function value
    fn calc_initial_cost(&mut self) -> Result<(), Error> {
        let tmp = self.cur_param();
        self.finit_b = Some(self.apply(&tmp)?);
        Ok(())
    }

    /// Calculate initial cost function value
    fn calc_initial_gradient(&mut self) -> Result<(), Error> {
        let tmp = self.cur_param();
        self.init_grad_b = Some(self.gradient(&tmp)?);
        Ok(())
    }

    /// Set initial alpha value
    fn set_initial_alpha(&mut self, alpha: f64) -> Result<(), Error> {
        if alpha <= 0.0 {
            return Err(ArgminError::InvalidParameter {
                text: "MoreThuenteLineSearch: Initial alpha must be > 0.".to_string(),
            }
            .into());
        }
        self.alpha = alpha;
        Ok(())
    }
}

impl<T, H, O> ArgminNextIter for MoreThuenteLineSearch<T, H, O>
where
    T: std::default::Default
        + Clone
        + std::fmt::Debug
        + ArgminSub<T, T>
        + ArgminDot<T, f64>
        + ArgminScaledAdd<T, f64, T>,
    H: Clone + std::default::Default,
    O: ArgminOperator<Parameters = T, OperatorOutput = f64, Hessian = H>,
{
    type Parameters = T;
    type OperatorOutput = f64;
    type Hessian = H;

    fn init(&mut self) -> Result<(), Error> {
        self.init_param = check_param!(
            self.init_param_b,
            "MoreThuenteLineSearch: Initial parameter not initialized. Call `set_initial_parameter`."
        );

        self.finit = check_param!(
            self.finit_b,
            "MoreThuenteLineSearch: Initial cost not computed. Call `set_initial_cost` or `calc_inital_cost`."
        );

        self.init_grad = check_param!(
            self.init_grad_b,
            "MoreThuenteLineSearch: Initial gradient not computed. Call `set_initial_grad` or `calc_inital_grad`."
        );

        self.search_direction = check_param!(
            self.search_direction_b,
            "MoreThuenteLineSearch: Search direction not initialized. Call `set_search_direction`."
        );

        self.dginit = self.init_grad.dot(&self.search_direction);

        // compute search direction in 1D
        if self.dginit >= 0.0 {
            return Err(ArgminError::ConditionViolated {
                text: "MoreThuenteLineSearch: Search direction must be a descent direction."
                    .to_string(),
            }
            .into());
        }

        self.stage1 = true;
        self.brackt = false;

        self.dgtest = self.ftol * self.dginit;
        self.width = self.stpmax - self.stpmin;
        self.width1 = 2.0 * self.width;
        self.f = self.finit;

        self.stp = Step::new(self.alpha, std::f64::NAN, std::f64::NAN);
        self.stx = Step::new(0.0, self.finit, self.dginit);
        self.sty = Step::new(0.0, self.finit, self.dginit);

        Ok(())
    }

    fn next_iter(&mut self) -> Result<ArgminIterationData<Self::Parameters>, Error> {
        // set the minimum and maximum steps to correspond to the present interval of uncertainty
        let mut info = 0;
        let (stmin, stmax) = if self.brackt {
            (self.stx.x.min(self.sty.x), self.stx.x.max(self.sty.x))
        } else {
            (
                self.stx.x,
                self.stp.x + self.xtrapf * (self.stp.x - self.stx.x),
            )
        };

        // alpha needs to be within bounds
        self.stp.x = self.stp.x.max(self.stpmin);
        self.stp.x = self.stp.x.min(self.stpmax);

        // If an unusual termination is to occur then let alpha be the lowest point obtained so
        // far.
        if (self.brackt && (self.stp.x <= stmin || self.stp.x >= stmax))
            || (self.brackt && (stmax - stmin) <= self.xtol * stmax)
            || self.infoc == 0
        {
            self.stp.x = self.stx.x;
        }

        // Evaluate the function and gradient at new stp.x and compute the directional derivative
        let new_param = self
            .init_param
            .scaled_add(&self.stp.x, &self.search_direction);
        self.f = self.apply(&new_param)?;
        let new_grad = self.gradient(&new_param)?;
        let f = self.f;
        self.set_cur_cost(f);
        self.set_cur_param(new_param);
        self.set_cur_grad(new_grad.clone());
        // self.stx.fx = new_cost;
        let dg = self.search_direction.dot(&new_grad);
        let ftest1 = self.finit + self.stp.x * self.dgtest;
        // self.stp.fx = new_cost;
        // self.stp.gx = dg;

        if (self.brackt && (self.stp.x <= stmin || self.stp.x >= stmax)) || self.infoc == 0 {
            info = 6;
        }

        if (self.stp.x - self.stpmax).abs() < std::f64::EPSILON
            && self.f <= ftest1
            && dg <= self.dgtest
        {
            info = 5;
        }

        if (self.stp.x - self.stpmin).abs() < std::f64::EPSILON
            && (self.f > ftest1 || dg >= self.dgtest)
        {
            info = 4;
        }

        if self.brackt && stmax - stmin <= self.xtol * stmax {
            info = 2;
        }

        if self.f <= ftest1 && dg.abs() <= self.gtol * (-self.dginit) {
            info = 1;
        }

        if info != 0 {
            self.set_termination_reason(TerminationReason::LineSearchConditionMet);
            let out = ArgminIterationData::new(self.cur_param(), self.cur_cost());
            return Ok(out);
        }

        if self.stage1 && self.f <= ftest1 && dg >= self.ftol.min(self.gtol) * self.dginit {
            self.stage1 = false;
        }

        if self.stage1 && self.f <= self.stp.fx && self.f > ftest1 {
            let fm = self.f - self.stp.x * self.dgtest;
            let fxm = self.stx.fx - self.stx.x * self.dgtest;
            let fym = self.sty.fx - self.sty.x * self.dgtest;
            let dgm = dg - self.dgtest;
            let dgxm = self.stx.gx - self.dgtest;
            let dgym = self.sty.gx - self.dgtest;

            let (stx1, sty1, stp1, brackt1, _stmin, _stmax, infoc) = cstep(
                Step::new(self.stx.x, fxm, dgxm),
                Step::new(self.sty.x, fym, dgym),
                Step::new(self.stp.x, fm, dgm),
                self.brackt,
                stmin,
                stmax,
            );

            self.stx.x = stx1.x;
            self.sty.x = sty1.x;
            self.stp.x = stp1.x;
            self.stx.fx += stx1.x * self.dgtest;
            self.sty.fx += sty1.x * self.dgtest;
            self.stx.gx += self.dgtest;
            self.sty.gx += self.dgtest;
            self.brackt = brackt1;
            self.stp = stp1;
            self.infoc = infoc;
        } else {
            let (stx1, sty1, stp1, brackt1, _stmin, _stmax, infoc) = cstep(
                self.stx.clone(),
                self.sty.clone(),
                Step::new(self.stp.x, self.f, dg),
                self.brackt,
                stmin,
                stmax,
            );
            self.stx = stx1;
            self.sty = sty1;
            self.stp = stp1;
            self.f = self.stp.fx;
            // dg = self.stp.gx;
            self.brackt = brackt1;
            self.infoc = infoc;
        }

        if self.brackt {
            if (self.sty.x - self.stx.x).abs() >= 0.66 * self.width1 {
                self.stp.x = self.stx.x + 0.5 * (self.sty.x - self.stx.x);
            }
            self.width1 = self.width;
            self.width = (self.sty.x - self.stx.x).abs();
        }

        let new_param = self
            .init_param
            .scaled_add(&self.stp.x, &self.search_direction);
        let out = ArgminIterationData::new(new_param, self.stp.fx);
        Ok(out)
    }
}

fn cstep(
    stx: Step,
    sty: Step,
    stp: Step,
    brackt: bool,
    stpmin: f64,
    stpmax: f64,
) -> (Step, Step, Step, bool, f64, f64, usize) {
    let mut info: usize = 0;
    let bound: bool;
    let mut stpf: f64;
    let stpc: f64;
    let stpq: f64;
    let mut brackt = brackt;

    // check inputs
    if (brackt && (stp.x <= stx.x.min(sty.x) || stp.x >= stx.x.max(sty.x)))
        || stx.gx * (stp.x - stx.x) >= 0.0
        || stpmax < stpmin
    {
        return (stx, sty, stp, brackt, stpmin, stpmax, info);
    }

    // determine if the derivatives have opposite sign
    let sgnd = stp.gx * (stx.gx / stx.gx.abs());

    if stp.fx > stx.fx {
        // First case. A higher function value. The minimum is bracketed. If the cubic step is closer to
        // stx.x than the quadratic step, the cubic step is taken, else the average of the cubic and
        // the quadratic steps is taken.
        info = 1;
        bound = true;
        let theta = 3.0 * (stx.fx - stp.fx) / (stp.x - stx.x) + stx.gx + stp.gx;
        let tmp = vec![theta, stx.gx, stp.gx];
        let s = tmp.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let mut gamma = s * ((theta / s).powi(2) - (stx.gx / s) * (stp.gx / s)).sqrt();
        if stp.x < stx.x {
            gamma = -gamma;
        }

        let p = (gamma - stx.gx) + theta;
        let q = ((gamma - stx.gx) + gamma) + stp.gx;
        let r = p / q;
        stpc = stx.x + r * (stp.x - stx.x);
        stpq = stx.x
            + ((stx.gx / ((stx.fx - stp.fx) / (stp.x - stx.x) + stx.gx)) / 2.0) * (stp.x - stx.x);
        if (stpc - stx.x).abs() < (stpq - stx.x).abs() {
            stpf = stpc;
        } else {
            stpf = stpc + (stpq - stpc) / 2.0;
        }
        brackt = true;
    } else if sgnd < 0.0 {
        // Second case. A lower function value and derivatives of opposite sign. The minimum is
        // bracketed. If the cubic step is closer to stx.x than the quadtratic (secant) step, the
        // cubic step is taken, else the quadratic step is taken.
        info = 2;
        bound = false;
        let theta = 3.0 * (stx.fx - stp.fx) / (stp.x - stx.x) + stx.gx + stp.gx;
        let tmp = vec![theta, stx.gx, stp.gx];
        let s = tmp.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let mut gamma = s * ((theta / s).powi(2) - (stx.gx / s) * (stp.gx / s)).sqrt();
        if stp.x > stx.x {
            gamma = -gamma;
        }
        let p = (gamma - stp.gx) + theta;
        let q = ((gamma - stp.gx) + gamma) + stx.gx;
        let r = p / q;
        stpc = stp.x + r * (stx.x - stp.x);
        stpq = stp.x + (stp.gx / (stp.gx - stx.gx)) * (stx.x - stp.x);
        if (stpc - stp.x).abs() > (stpq - stp.x).abs() {
            stpf = stpc;
        } else {
            stpf = stpq;
        }
        brackt = true;
    } else if stp.gx.abs() < stx.gx.abs() {
        // Third case. A lower function value, derivatives of the same sign, and the magnitude of
        // the derivative decreases. The cubic step is only used if the cubic tends to infinity in
        // the direction of the step or if the minimum of the cubic is beyond stp.x. Otherwise the
        // cubic step is defined to be either stpmin or stpmax. The quadtratic (secant) step is
        // also computed and if the minimum is bracketed then the step closest to stx.x is taken,
        // else the step farthest away is taken.
        info = 3;
        bound = true;
        let theta = 3.0 * (stx.fx - stp.fx) / (stp.x - stx.x) + stx.gx + stp.gx;
        let tmp = vec![theta, stx.gx, stp.gx];
        let s = tmp.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        // the case gamma == 0 only arises if the cubic does not tend to infinity in the direction
        // of the step.

        let mut gamma = s * 0.0f64
            .max((theta / s).powi(2) - (stx.gx / s) * (stp.gx / s))
            .sqrt();
        if stp.x > stx.x {
            gamma = -gamma;
        }

        let p = (gamma - stp.gx) + theta;
        let q = (gamma + (stx.gx - stp.gx)) + gamma;
        let r = p / q;
        if r < 0.0 && gamma != 0.0 {
            stpc = stp.x + r * (stx.x - stp.x);
        } else if stp.x > stx.x {
            stpc = stpmax;
        } else {
            stpc = stpmin;
        }
        stpq = stp.x + (stp.gx / (stp.gx - stx.gx)) * (stx.x - stp.x);
        if brackt {
            if (stp.x - stpc).abs() < (stp.x - stpq).abs() {
                stpf = stpc;
            } else {
                stpf = stpq;
            }
        } else if (stp.x - stpc).abs() > (stp.x - stpq).abs() {
            stpf = stpc;
        } else {
            stpf = stpq;
        }
    } else {
        // Fourth case. A lower function value, derivatives of the same sign, and the magnitued of
        // the derivative does not decrease. If the minimum is not bracketed, the step is either
        // stpmin or stpmax, else the cubic step is taken.
        info = 4;
        bound = false;
        if brackt {
            let theta = 3.0 * (stp.fx - sty.fx) / (sty.x - stp.x) + sty.gx + stp.gx;
            let tmp = vec![theta, sty.gx, stp.gx];
            let s = tmp.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let mut gamma = s * ((theta / s).powi(2) - (sty.gx / s) * (stp.gx / s)).sqrt();
            if stp.x > sty.x {
                gamma = -gamma;
            }
            let p = (gamma - stp.gx) + theta;
            let q = ((gamma - stp.gx) + gamma) + sty.gx;
            let r = p / q;
            stpc = stp.x + r * (sty.x - stp.x);
            stpf = stpc;
        } else if stp.x > stx.x {
            stpf = stpmax;
        } else {
            stpf = stpmin;
        }
    }
    // Update the interval of uncertainty. This update does not depend on the new step or the case
    // analysis above.

    let mut stx_o = stx.clone();
    let mut sty_o = sty.clone();
    let mut stp_o = stp.clone();
    if stp_o.fx > stx_o.fx {
        sty_o = Step::new(stp_o.x, stp_o.fx, stp_o.gx);
    } else {
        if sgnd < 0.0 {
            sty_o = Step::new(stx_o.x, stx_o.fx, stx_o.gx);
        }
        stx_o = Step::new(stp_o.x, stp_o.fx, stp_o.gx);
    }

    // compute the new step and safeguard it.

    stpf = stpmax.min(stpf);
    stpf = stpmin.max(stpf);

    stp_o.x = stpf;
    if brackt && bound {
        if sty_o.x > stx_o.x {
            stp_o.x = stp_o.x.min(stx_o.x + 0.66 * (sty_o.x - stx_o.x));
        } else {
            stp_o.x = stp_o.x.max(stx_o.x + 0.66 * (sty_o.x - stx_o.x));
        }
    }

    (stx_o, sty_o, stp_o, brackt, stpmin, stpmax, info)
}
