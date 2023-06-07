use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Add;
use std::process::Output;
use std::rc::Rc;

/// A trait defining behaviour for a parameter which can be modulated;
/// Must store a value, and apply modulation around a base, and in a custom range.
/// All the getters and setters must return f32
pub trait Modulable {
    /// Get the value of the parameter from a shared reference
    fn get_value(&self) -> f32;
    /// Set the value of the parameter into a shared reference
    fn set_value(&mut self, value: f32);
    /// Adjust the value stored in the parameter, by adding the base value,
    /// and push that value to the shared reference
    fn adjust_with_base(&mut self);
    /// Get the upper boundary of the parameter
    fn get_upper(&self) -> f32;
    /// Get the lower boundary of the parameter
    fn get_lower(&self) -> f32;
}

/// A trait defining behaviour for a modulator object, which has a getter for its current value, and a reset function to reset the modulation.
pub trait Modulator {
    /// Get the value for the current timestamp, should not change the value by itself
    fn get_value(&mut self) -> f32;
    /// Set the structs value to its next value, called after all the accessors of this object have called get_value
    fn advance(&mut self);
    /// Reset the modulation signal, examples: restarting an lfo, re-triggering an envelope, resampling s&h, etc...
    fn reset(&mut self);
}

/// A generic struct for numeric parameters.
///
/// * T must implement `Copy`, usually by default if primitive
///
/// * T must implement `Into<f32>` so that it can be cast to that in the value getter
///
/// * T must implement `From<f32>` so that the setter can take an f32 and convert it to T
///
/// * `value` stores the current value of that parameter - which is synced to its reference. Does not account for base
///
/// * `base` is the mid point of modulation, meaning the modulation ranges from `base - 1/2 depth` to `base + 1/2 depth` in LFOs
/// or from `base` to `base + 1/2 depth`
///
/// * `upper` and `lower` are the upper and lower bounds of modulation.
/// For example, if the lower bound is 0 and a value of -10 is attempted, it will simply set the value to 0.
///
/// * `param_ref` stores a mutable reference to the parameter this is associated (a variable in a struct or otherwise)
/// and updates the references value each time the structs value is modified, using a Cell type.
///
/// The parameter of a struct which this corresponds to needs to get the value from the cell each time the modulation occurs.
struct NumericParameter<T>
where
    T: Copy + Into<f32> + From<f32> + Add<Output = T> + PartialOrd,
{
    value: T,
    base: f32,
    upper: T,
    lower: T,
    param_ref: Cell<T>,
}

impl<T> Modulable for NumericParameter<T>
where
    T: Copy + Into<f32> + From<f32> + Add<Output = T> + PartialOrd,
{
    fn get_value(&self) -> f32 {
        self.param_ref.get().into()
    }
    fn set_value(&mut self, value: f32) {
        self.value = self.value + value.into();
    }
    fn adjust_with_base(&mut self) {
        let adjusted = self.value + self.base.into();
        if adjusted > self.upper {
            self.value = self.upper;
        }
        if adjusted < self.lower {
            self.value = self.lower;
        } else {
            self.value = adjusted;
        }
        self.param_ref.replace(self.value);
        self.value = 0.0.into();
    }
    fn get_upper(&self) -> f32 {
        self.upper.into()
    }
    fn get_lower(&self) -> f32 {
        self.lower.into()
    }
}

/// The same as a numeric parameter, but without the numeric fields - base, upper, lower, depth
struct BoolParameter {
    value: bool,
    param_ref: Cell<bool>,
}

impl Modulable for BoolParameter {
    fn set_value(&mut self, value: f32) {
        self.value = match value.clamp(0.0, 1.0) {
            x if x >= 0.5 => true,
            x if x < 0.5 => false,
            _ => panic!("clamp function didn't work"),
        };
    }
    fn adjust_with_base(&mut self) {
        self.param_ref.replace(self.value);
        self.value = false;
    }
    fn get_upper(&self) -> f32 {
        1.0
    }
    fn get_lower(&self) -> f32 {
        0.0
    }
    fn get_value(&self) -> f32 {
        match self.param_ref.get() {
            true => 1.0,
            false => 0.0,
        }
    }
}

/// Struct holding a Modulator - Parameter pair.
///
/// The src is the modulation source.
///
/// The dst is the modulation destination.
///
/// The depth is the effective amplitude of the modulation,
/// meaning the range of the modulation should be from 0 to depth, or in some cases -depth/2 to depth/2
struct Modulation {
    src: Rc<RefCell<Box<dyn Modulator>>>,
    dst: Rc<RefCell<Box<dyn Modulable>>>,
    depth: f32,
}

impl Modulation {
    fn apply_modulation(&mut self) {
        // The notation here of *self.src.borrow_mut() is used because borrow_mut returns an &mut type
        // and so brackets are used to dereference that before calling the get_value method
        let mod_value = (*self.src.borrow_mut()).get_value() * self.depth;
        (*self.dst.borrow_mut()).set_value(mod_value);
    }
}

/// Struct which manages multiple modulations, and allows methods to be called on them.
/// ## Attributes:
/// * `modulations`: A vector of `Modulation` instances, which is used to iteratively apply all the modulations for that tick.
///
/// * `modulator_map`: A hashmap identifying and registering modulators (implementing the trait) by a string ID
///
/// * `parameter_map`: A hashmap identifying and registering modulable parameters (implementing the trait) by a string ID
///
/// All values in the hashmap are reference counted RefCells, so that multiple modulations may contain each
///
/// This creates a M : N relationship between modulable and modulator, but a 1 : 1 relationship between modulable and struct parameter in practice.
struct ModManager {
    modulations: Vec<Modulation>,
    modulator_map: HashMap<String, Rc<RefCell<Box<dyn Modulator>>>>,
    parameter_map: HashMap<String, Rc<RefCell<Box<dyn Modulable>>>>,
}

impl ModManager {
    /// Constructor for a new mod manager, with uninitialised fields
    pub fn new() -> Self {
        Self {
            modulations: Vec::new(),
            modulator_map: HashMap::new(),
            parameter_map: HashMap::new(),
        }
    }

    /// Register a modulation source, boxed as it is a DST, and assign it a unique string ID
    pub fn register_source(&mut self, name: &str, source: Box<dyn Modulator>) {
        self.modulator_map
            .insert(String::from(name), Rc::new(RefCell::new(source)));
    }

    /// Register a modulation destination, boxed as it is a DST, and assign it a unique string ID
    pub fn register_destination(&mut self, name: &str, destination: Box<dyn Modulable>) {
        self.parameter_map
            .insert(String::from(name), Rc::new(RefCell::new(destination)));
    }

    /// Register a modulation object, by the string identifiers of a source and destination.
    /// Will clone the reference counters so that the modulation may use sources and or destinations already used in other modulations
    pub fn add_modulation(&mut self, src: &str, dst: &str, depth: f32) {
        self.modulations.push(Modulation {
            src: Rc::clone(
                self.modulator_map
                    .get(src)
                    .expect(format!("Modulation source: {} does not exist", src).as_str()),
            ),
            dst: Rc::clone(
                self.parameter_map
                    .get(dst)
                    .expect(format!("Modulation destination: {} does not exist", dst).as_str()),
            ),
            depth,
        })
    }

    /// Register modulable parameters from a parameter manager struct, by cloning their values into this objects hashmap
    pub fn register_from_parameters(&mut self, parameters: &ParameterManager) {
        for (name, rc) in parameters.get_map().iter() {
            self.parameter_map.insert(name.clone(), Rc::clone(&rc));
        }
    }

    /// function which, for each modulation in the modulation map, applies the modulation to the parameter.
    /// Next, updates the value with that parameters base and pushes its result to the shared Cell.
    /// Finally, advances each modulator, which is done last, because each time this is called on a modulator, it needs to return the same value.  
    pub fn do_modulation(&mut self) {
        for modulation in self.modulations.iter_mut() {
            modulation.apply_modulation();
        }
        for modulation in self.modulations.iter_mut() {
            modulation.dst.borrow_mut().adjust_with_base();
            (*modulation.src.borrow_mut()).advance();
        }
    }

    /// Set a parameters value, by ID
    pub fn set_value(&mut self, id: &str, value: f32) {
        self.parameter_map
            .get(id)
            .unwrap()
            .borrow_mut()
            .set_value(value);
    }

    /// Get a parameters value, by ID
    pub fn get_value(&self, id: &str) -> f32 {
        self.parameter_map
            .get(id)
            .expect("ID does not exist in parameter map")
            .borrow()
            .get_value()
    }

    /// Get a parameters upper bound, by ID
    pub fn get_upper(&self, id: &str) -> f32 {
        self.parameter_map
            .get(id)
            .expect("ID does not exist in parameter map")
            .borrow()
            .get_upper()
    }

    /// Get a parameters lower bound, by ID
    pub fn get_lower(&self, id: &str) -> f32 {
        self.parameter_map
            .get(id)
            .expect("ID does not exist in parameter map")
            .borrow()
            .get_lower()
    }
}

/// Struct which contains the parameter objects for the particular struct is is associated with.
/// Stored by reference counted RefCell for compatibility with the ModManager.
struct ParameterManager {
    map: HashMap<String, Rc<RefCell<Box<dyn Modulable>>>>,
}

impl ParameterManager {
    /// Constructor for uninitialized parameter manager.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Register a parameter, boxed as it is a DST, and assign it a unique string ID
    pub fn register_parameter(&mut self, name: &str, parameter: Box<dyn Modulable>) {
        self.map
            .insert(String::from(name), Rc::new(RefCell::new(parameter)));
    }

    /// Get the instances parameter map by reference
    pub fn get_map(&self) -> &HashMap<String, Rc<RefCell<Box<dyn Modulable>>>> {
        &self.map
    }

    /// Set a parameters value, by ID
    pub fn set_value(&mut self, id: &str, value: f32) {
        self.map
            .get(id)
            .expect("ID does not exist in manager")
            .borrow_mut()
            .set_value(value);
    }

    /// Get a parameters value, by ID
    pub fn get_value(&self, id: &str) -> f32 {
        self.map
            .get(id)
            .expect("ID does not exist in manager")
            .borrow()
            .get_value()
    }

    /// Get a parameters upper bound, by ID
    pub fn get_upper(&self, id: &str) -> f32 {
        self.map
            .get(id)
            .expect("ID does not exist in manager")
            .borrow()
            .get_upper()
    }

    /// Get a parameters lower bound, by ID
    pub fn get_lower(&self, id: &str) -> f32 {
        self.map
            .get(id)
            .expect("ID does not exist in manager")
            .borrow()
            .get_lower()
    }
}

/// Demo struct for testing the system, with 3 fields of different types
struct ParameterContainer {
    field1: f32,
    field2: i16,
    field3: bool,
}

/// A simple struct to test the Modulator trait which simply returns a static value.
struct Incrementer {
    increment: f32,
}
impl Modulator for Incrementer {
    fn get_value(&mut self) -> f32 {
        self.increment
    }
    fn advance(&mut self) {}
    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use crate::delay_line::{DelayLine, StereoDelay};
    use crate::lfo::{LFOMode, MMLFO};
    use crate::modulation::{
        BoolParameter, Incrementer, ModManager, Modulator, NumericParameter, ParameterContainer,
        ParameterManager,
    };
    use crate::samples::{IntSamples, PhonicMode, Samples};
    use crate::{load_wav, write_wav};
    use std::cell::Cell;
    use std::collections::HashMap;

    #[test]
    fn test_modulation_creation() {
        let mut manager = ModManager::new();
        let mut params = ParameterContainer {
            field1: 1.0,
            field2: 64,
            field3: false,
        };
        let modulator1 = Incrementer { increment: 0.1 };
        let modulator2 = Incrementer { increment: 2.0 };
        let mut field1_parameter = NumericParameter::<f32> {
            value: 0.0,
            base: 1.0,
            lower: 0.0,
            upper: 2.0,
            param_ref: Cell::new(params.field1),
        };
        // let mut field2_parameter = NumericParameter::<i16> {
        //     value: 64,
        //     base: 64.0,
        //     lower: 0,
        //     upper: 128,
        //     param_ref: &mut params.field2,
        //     depth: 64.0,
        // };
        let mut field3_parameter = BoolParameter {
            value: false,
            param_ref: Cell::new(params.field3),
        };
        // Need to implement copy / clone for the boxing of parameter, or refactor to use a reference.
        // Potentially find way to make Rc<RefCell<Box<&T>>> a little less ugly
        manager.register_destination("params_field1", Box::new(field1_parameter));
        manager.register_destination("params_field3", Box::new(field3_parameter));
        manager.register_source("increment_modulator1", Box::new(modulator1));
        manager.register_source("increment_modulator2", Box::new(modulator2));
        manager.add_modulation("increment_modulator1", "params_field1", 1.0);
        let mut values_history: Vec<f32> = Vec::new();
        for _ in 0..4 {
            manager.do_modulation();
            values_history.push(
                // This needs cleaning up in a method
                manager
                    .parameter_map
                    .get("params_field1")
                    .unwrap()
                    .borrow()
                    .get_value(),
            );
        }
        for v in values_history {
            println!("value: {}", v);
        }
    }

    #[test]
    fn test_parameter_registry() {
        let mut manager = ModManager::new();
        let mut parameter_manager = ParameterManager::new();
        let mut params = ParameterContainer {
            field1: 35.0,
            field2: 186,
            field3: true,
        };
        let mut field1_parameter = NumericParameter::<f32> {
            value: 0.0,
            base: 35.0,
            lower: 0.0,
            upper: 70.0,
            param_ref: Cell::new(params.field1),
        };
        let mut field3_parameter = BoolParameter {
            value: true,
            param_ref: Cell::new(params.field3),
        };
        parameter_manager.register_parameter("field1", Box::new(field1_parameter));
        parameter_manager.register_parameter("field3", Box::new(field3_parameter));
        manager.register_from_parameters(&parameter_manager);

        for parameter in manager.parameter_map.iter() {
            println!("{}: {}", parameter.0, parameter.1.borrow().get_value())
        }
        println!("\nChecking parameter_manager maintains its entries\n");
        for parameter in parameter_manager.map.iter() {
            println!("{}: {}", parameter.0, parameter.1.borrow().get_value())
        }
        println!("\n...modifying some values...\n");
        parameter_manager.set_value("field1", 1.0);

        println!("Main manager:\n");
        for parameter in manager.parameter_map.iter() {
            println!("{}: {}", parameter.0, parameter.1.borrow().get_value())
        }
        println!("\nParameter manager:\n");
        for parameter in parameter_manager.map.iter() {
            println!("{}: {}", parameter.0, parameter.1.borrow().get_value())
        }
    }

    #[test]
    fn test_with_chorus() {
        let mut delay = StereoDelay::new(44100.0, 0.0015, 0.0020, 0.65, 0.5);
        let mut lfo = MMLFO::new(false, LFOMode::Sine);
        lfo.set_frequency_hz(0.25);

        let mut delay_seconds_l_p = NumericParameter {
            value: 0.0_f32,
            base: 0.002,
            upper: 0.0035,
            lower: 0.0005,
            param_ref: Cell::new(0.002),
        };
        let mut delay_seconds_r_p = NumericParameter {
            value: 0.0_f32,
            base: 0.0025,
            upper: 0.0040,
            lower: 0.001,
            param_ref: Cell::new(0.0025),
        };

        let mut delay_parameters = ParameterManager::new();
        let mut mod_manager = ModManager::new();

        delay_parameters.register_parameter("delay_seconds_l", Box::new(delay_seconds_l_p));
        delay_parameters.register_parameter("delay_seconds_r", Box::new(delay_seconds_r_p));
        mod_manager.register_from_parameters(&delay_parameters);
        mod_manager.register_source("lfo", Box::new(lfo));
        mod_manager.add_modulation("lfo", "delay_seconds_l", 0.005);
        mod_manager.add_modulation("lfo", "delay_seconds_r", 0.005);

        let input = load_wav("tests/kalimba.wav").unwrap();
        let stereo = IntSamples::new(input);

        let mut out: Vec<i16> = Vec::new();

        for (left, right) in stereo.get_frames() {
            let (l, r) = delay.process(left as f32, right as f32, false, false);
            out.push(l as i16);
            out.push(r as i16);
            mod_manager.do_modulation();
            delay.set_time_left(mod_manager.get_value("delay_seconds_l"));
            delay.set_time_right(mod_manager.get_value("delay_seconds_r"));
        }

        write_wav(
            "tests/debug/chorus_with_mod_manager.wav",
            out,
            PhonicMode::Stereo,
        );
    }
}
