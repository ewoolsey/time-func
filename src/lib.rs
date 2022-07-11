#![feature(is_sorted)]
use chrono::{DateTime, Duration, Utc};
use plotters::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// A vector of data points that we can use to analyse time series data.
/// Each data point is valid from the previous timestamp until its current timestamp.
/// If it is the first point of data then it is only valid for a total time of 0.
/**
```text
                .B
          |    /\    |
          |   /  \   |
          |  /    \  |
          | /      \ |
         A./        \.C
__________|__________|_________
Invalid   |  Valid   |  Invalid
_______________________________
          .     .    .
          A     B    C
```
*/
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeFunc(pub Vec<(DateTime<Utc>, f64)>);

impl From<Vec<(DateTime<Utc>, f64)>> for TimeFunc {
    fn from(input: Vec<(DateTime<Utc>, f64)>) -> Self { TimeFunc(input) }
}

impl TimeFunc {
    /// Creates an empty TimeFunc
    pub fn new() -> Self { TimeFunc(vec![]) }

    /// Gets the average of the function from time, to time - duration
    /// This function isn't perfects acurate as it uses the closest available indeces, and does not interpolate
    pub fn get_moving_average(&self, time: DateTime<Utc>, duration: Duration) -> f64 {
        let start_index = self.get_index_safe(&(time - duration));
        //if start_index == 0 { start_index = 1 }
        let end_index = self.get_index_safe(&time);
        if start_index == end_index {
            return self.0[end_index].1;
        }
        if end_index == 0 {
            return self.0[0].1;
        }
        let mut sum = 0.0;
        let mut total_duration = Duration::seconds(0);
        for i in start_index..end_index {
            let this = self.0[i];
            let next = self.0[i + 1];
            let duration = next.0 - this.0;
            total_duration = total_duration + duration;
            sum += this.1 * duration.num_seconds() as f64;
        }
        sum / total_duration.num_seconds() as f64
    }

    /// Gets the average of the function from time, to time - duration
    /// This function interpolates the function and uses exact positions
    pub fn get_integral_interpolated(&self, time: DateTime<Utc>, duration: Duration) -> f64 {
        let start_time = time - duration;
        let start_point = (start_time, self.get_value_interpolated(&start_time));
        let end_point = (time, self.get_value_interpolated(&time));
        let start_index = self.get_index_above(&start_time);
        let end_index = self.get_index_below(&time);
        let mut current_index = start_index;
        let mut integral = 0.0;
        integral += get_integral(start_point, self.0[start_index]);
        while current_index < end_index {
            integral += get_integral(self.0[current_index], self.0[current_index + 1]);
            current_index += 1;
            println!("here");
        }
        println!("start_index: {}", start_index);
        println!("end_index: {}", end_index);
        integral += get_integral(self.0[end_index], end_point);
        integral
    }

    /// Gets the linear interpolated average of the function
    pub fn get_average_interpolated(&self, time: DateTime<Utc>, duration: Duration) -> f64 {
        let integral = self.get_integral_interpolated(time, duration);
        integral / duration.num_seconds() as f64
    }

    /// Get's the normalized RMS
    /// Does not use interpolation as of yet
    pub fn get_rms(&self, duration: Duration, time: DateTime<Utc>) -> f64 {
        let average = self.get_moving_average(time, duration);
        let end_index = self.get_index_safe(&time);
        let mut start_index = self.get_index_safe(&(time - duration));
        if end_index < 2 {
            return 0.0;
        }
        if end_index == start_index {
            start_index = end_index - 1;
        }
        let mut total_duration = Duration::seconds(0);
        let mut sum = 0.0;
        for i in start_index..end_index {
            let this = self.0[i];
            let next = self.0[i + 1];
            let duration = next.0 - this.0;
            let relative_square = ((this.1 - average) / average).powf(2.0);
            sum += relative_square * duration.num_seconds() as f64;
            total_duration = total_duration + duration;
        }
        (sum / total_duration.num_seconds() as f64).sqrt()
    }

    pub fn get_rms_timefunc(&self, duration: Duration) -> Self {
        let mut timefunc = TimeFunc::new();
        for i in 1..self.0.len() {
            let time = self.0[i].0;
            timefunc.push((time, self.get_rms(duration, time))).unwrap();
        }
        timefunc
    }

    pub fn get_inflation(&self, duration: Duration, time: DateTime<Utc>) -> f64 {
        let end_index = self.get_index_safe(&time);
        if end_index == 0 {
            return 0.0;
        }
        let mut start_index = self.get_index_safe(&(time - duration));
        if end_index == start_index {
            start_index = end_index - 1
        }
        let end = self.0[end_index];
        let start = self.0[start_index];
        let duration = end.0 - start.0;
        ((start.1 / end.1) - 1.0) * (365 * 24 * 60 * 60) as f64 / duration.num_seconds() as f64
    }

    pub fn get_inflation_interpolated(&self, duration: Duration, time: DateTime<Utc>) -> f64 {
        let start = self.get_value_interpolated(&(time - duration));
        let end = self.get_value_interpolated(&time);
        ((start / end) - 1.0) * (365 * 24 * 60 * 60) as f64 / duration.num_seconds() as f64
    }

    pub fn get_inflation_timefunc(&self, duration: Duration) -> Self {
        let mut timefunc = TimeFunc::new();
        for i in 1..self.0.len() {
            let time = self.0[i].0;
            timefunc
                .push((time, self.get_inflation(duration, time)))
                .unwrap();
        }
        timefunc
    }

    pub fn get_inflation_interpolated_timefunc(&self, duration: Duration) -> Self {
        let mut timefunc = TimeFunc::new();
        for i in 1..self.0.len() {
            let time = self.0[i].0;
            timefunc
                .push((time, self.get_inflation_interpolated(duration, time)))
                .unwrap();
        }
        timefunc
    }

    pub fn get_index(&self, time: &DateTime<Utc>) -> Result<usize, usize> {
        self.0.binary_search_by(|probe| probe.0.cmp(time))
    }

    pub fn get_index_safe(&self, time: &DateTime<Utc>) -> usize {
        match self.get_index(time) {
            Ok(index) => index,
            Err(index) => {
                let len = self.0.len();
                if index == len {
                    len - 1
                } else {
                    index
                }
            }
        }
    }

    pub fn get_index_above(&self, time: &DateTime<Utc>) -> usize {
        match self.get_index(time) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn get_index_below(&self, time: &DateTime<Utc>) -> usize {
        match self.get_index(time) {
            Ok(index) => index - 1,
            Err(index) => index - 1,
        }
    }

    /// Returns the fractional index of a specific time
    pub fn get_fractional_index(&self, time: &DateTime<Utc>) -> Result<f64, Box<dyn Error>> {
        match self.get_index(time) {
            Ok(index) => Ok(index as f64),
            Err(index) => {
                let len = self.0.len();
                if index == len || index == 0 {
                    // point falls out of range
                    return Err("Index is out of bounds".into())
                } else {
                    let lower_time = self.0[index - 1].0;
                    let upper_time = self.0[index].0;
                    let time_step = upper_time - lower_time;
                    let difference = *time - lower_time;
                    let fractional =
                        difference.num_seconds() as f64 / time_step.num_seconds() as f64;
                    let fractional_index = (index - 1) as f64 + fractional;
                    Ok(fractional_index)
                }
            }
        }
    }

    pub fn get_value(&self, time: DateTime<Utc>) -> f64 {
        let result = self.get_index(&time);
        match result {
            Ok(index) => self.0[index].1,
            Err(index) => {
                let len = self.0.len();
                if index == len {
                    self.0.last().unwrap().1
                } else {
                    self.0[index].1
                }
            }
        }
    }

    pub fn get_value_interpolated(&self, time: &DateTime<Utc>) -> f64 {
        let result = self.get_index(&time);
        match result {
            Ok(index) => self.0[index].1,
            // if value is not hit, the returned index is higher than the true value
            Err(index) => {
                if index == 0 {
                    return self.0[0].1;
                }
                let len = self.0.len();
                if index == len {
                    return self.0.last().unwrap().1;
                }
                let upper_index = index;
                let lower_index = index - 1;
                let upper_tuple = self.0[upper_index];
                let lower_tuple = self.0[lower_index];
                println!("lower index: {}", lower_index);
                println!("upper: {:?}", upper_tuple);
                println!("lower: {:?}", lower_tuple);

                let slope = (upper_tuple.1 - lower_tuple.1)
                    / (upper_tuple.0 - lower_tuple.0).num_seconds() as f64;
                slope * (time.to_owned() - lower_tuple.0).num_seconds() as f64 + lower_tuple.1
            }
        }
    }

    fn get_range(&self) -> std::ops::Range<f64> {
        let mut max = self.0[0].1;
        let mut min = self.0[0].1;
        for i in 1..self.0.len() {
            let val = self.0[i].1;
            if val > max {
                max = val
            } else if val < min {
                min = val
            }
        }
        min..max
    }

    fn get_domain(&self) -> std::ops::Range<DateTime<Utc>> {
        let first = self.0[0].0;
        let last = self.0.last().unwrap().0;
        first..last
    }

    pub fn push(&mut self, point: (DateTime<Utc>, f64)) -> Result<(), Box<dyn Error>> {
        match self.0.last() {
            Some(last) => {
                if last.0 >= point.0 {
                    return Err("Attempting to add point to TimeFunc that is out of order.".into());
                }
            }
            None => {}
        }
        self.0.push(point);
        Ok(())
    }

    /// Verifies that the timefunc is valid.
    /// Checks to ensure the list is sorted and deduped.
    pub fn verify(&self) -> Result<(), Box<dyn Error>> {
        if self.0.is_sorted_by(|a, b| Some(a.0.cmp(&b.0))) && self.is_deduped() {
            Ok(())
        } else {
            Err("TimeFunc is invalid.".into())
        }
    }

    /// Executed dedup
    /// Assumes vec is sorted.
    pub fn dedup(&mut self) { self.0.dedup_by(|a, b| a.0.eq(&b.0)); }

    /// Verifies that the function is deduped
    /// Assumes vec is sorted is sorted.
    pub fn is_deduped(&self) -> bool {
        for i in 1..self.0.len() {
            if self.0[i].0 == self.0[i - 1].0 {
                return false;
            }
        }
        true
    }

    /// Fixes the TimeFunc by sorting it.
    pub fn repair(&mut self) {
        self.0.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        self.dedup();
    }

    pub fn draw(&self, title: String) -> Result<(), Box<dyn std::error::Error>> {
        let filename = format!("images/{}.png", title);
        let style = ("sans-serif", 25).into_font();
        let root = BitMapBackend::new(&filename, (1200, 800)).into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(title, ("sans-serif", 50).into_font())
            .margin(10)
            .x_label_area_size(60)
            .y_label_area_size(80)
            .right_y_label_area_size(80)
            .build_cartesian_2d(self.get_domain().yearly(), self.get_range())?;

        chart
            .configure_mesh()
            .x_label_style(style.clone())
            .y_label_style(style)
            .draw()?;

        chart.draw_series(LineSeries::new(self.0.clone(), &BLUE))?;

        Ok(())
    }
}

/// Gets the area under the interpolated curve
/// Assumes points are ordered properly
fn get_integral(a: (DateTime<Utc>, f64), b: (DateTime<Utc>, f64)) -> f64 {
    let duration = b.0 - a.0;
    let average = get_average(b.1, a.1);
    duration.num_seconds() as f64 * average
}

/// Simply gets the average of two f64s
fn get_average(a: f64, b: f64) -> f64 { (a + b) / 2.0 }

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_moving_average() {
        let mut time_func = TimeFunc::new();
        let start_time = Utc::now();
        let time_step = Duration::seconds(60);
        time_func.push((start_time, 1.0)).unwrap();
        time_func.push((start_time + time_step, 1.0)).unwrap();
        time_func.push((start_time + time_step * 2, 1.0)).unwrap();

        let av = time_func.get_moving_average(start_time + time_step * 2, time_step * 2);
        assert_eq!(av, 1.0)
    }

    #[test]
    fn test_get_fractional_index() {
        let mut time_func = TimeFunc::new();
        let start_time = Utc::now();
        let time_step = Duration::seconds(60);
        time_func.push((start_time, 1.0)).unwrap();
        time_func.push((start_time + time_step, 1.0)).unwrap();
        time_func.push((start_time + time_step * 2, 1.0)).unwrap();

        let fractional_index = time_func
            .get_fractional_index(&(start_time + time_step / 2))
            .unwrap();
        assert_eq!(fractional_index, 0.5);
        let fractional_index = time_func.get_fractional_index(&(start_time - time_step / 2));
        assert!(fractional_index.is_err());
    }

    #[test]
    fn test_get_value_interpolated() {
        let mut time_func = TimeFunc::new();
        let start_time = Utc::now();
        let time_step = Duration::seconds(60);
        time_func.push((start_time, 1.0)).unwrap();
        time_func.push((start_time + time_step, 2.0)).unwrap();
        time_func.push((start_time + time_step * 2, 1.0)).unwrap();

        let value = time_func.get_value_interpolated(&(start_time + time_step / 2));
        assert_eq!(value, 1.5);
    }

    #[test]
    fn test_get_integral_interpolated() {
        let mut time_func = TimeFunc::new();
        let start_time = Utc::now();
        let time_step = Duration::seconds(60);
        time_func.push((start_time, 1.0)).unwrap();
        time_func.push((start_time + time_step, 2.0)).unwrap();
        time_func.push((start_time + time_step * 2, 1.0)).unwrap();

        let integral =
            time_func.get_integral_interpolated(start_time + time_step * 2, time_step * 2);
        println!("integral: {}", integral);
        assert_eq!(integral, 1.5 * 60.0 + 1.5 * 60.0);
    }
}
