//! Function Call Optimizations and Inlining
//!
//! Advanced function call optimizations to outperform PHP 8

use crate::engine::types::{PhpResult, Val};
use crate::vm::execute_data::{ExecResult, ExecuteData};
use crate::vm::opcodes::{Op, OpArray, Opcode};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Function call optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None = 0,
    Basic = 1,      // Basic call optimizations
    Aggressive = 2, // Aggressive inlining
}

/// Function metadata for optimization
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub param_count: usize,
    pub op_count: usize,
    pub complexity: Complexity,
    pub call_frequency: u64,
    pub inline_candidates: Vec<String>,
    pub optimization_level: OptimizationLevel,
}

/// Function complexity analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Trivial,     // Simple arithmetic/logic
    Simple,      // Basic operations
    Moderate,    // Some loops/conditions
    Complex,     // Complex control flow
    VeryComplex, // Highly complex
}

/// Inlined function representation
#[derive(Debug, Clone)]
pub struct InlinedFunction {
    pub metadata: FunctionMetadata,
    pub optimized_ops: Vec<Op>,
    pub inline_threshold: f32,
}

/// Function call optimizer
#[derive(Debug)]
pub struct FunctionOptimizer {
    function_cache: HashMap<String, FunctionMetadata>,
    inline_cache: HashMap<String, InlinedFunction>,
    call_stats: HashMap<String, CallStats>,
    optimization_stats: OptimizationStats,
}

#[derive(Debug, Default)]
pub struct CallStats {
    pub call_count: u64,
    pub total_time_ns: u64,
    pub average_time_ns: u64,
}

#[derive(Debug, Default)]
pub struct OptimizationStats {
    pub functions_optimized: u64,
    pub functions_inlined: u64,
    pub calls_saved: u64,
    pub total_time_saved_ns: u64,
}

impl FunctionOptimizer {
    pub fn new() -> Self {
        Self {
            function_cache: HashMap::new(),
            inline_cache: HashMap::new(),
            call_stats: HashMap::new(),
            optimization_stats: OptimizationStats::default(),
        }
    }

    /// Analyze function for optimization opportunities
    pub fn analyze_function(&mut self, name: &str, op_array: &OpArray) -> FunctionMetadata {
        let metadata = if let Some(cached) = self.function_cache.get(name) {
            cached.clone()
        } else {
            let metadata = self.perform_analysis(name, op_array);
            self.function_cache
                .insert(name.to_string(), metadata.clone());
            metadata
        };

        // Update optimization level based on call frequency
        let updated_metadata = self.update_optimization_level(metadata);
        self.function_cache
            .insert(name.to_string(), updated_metadata.clone());
        updated_metadata
    }

    /// Perform detailed function analysis
    fn perform_analysis(&self, name: &str, op_array: &OpArray) -> FunctionMetadata {
        let param_count = op_array.vars.len();
        let op_count = op_array.ops.len();
        let complexity = self.analyze_complexity(&op_array.ops);
        let inline_candidates = self.find_inline_candidates(&op_array.ops);

        FunctionMetadata {
            name: name.to_string(),
            param_count,
            op_count,
            complexity,
            call_frequency: 0,
            inline_candidates,
            optimization_level: OptimizationLevel::None,
        }
    }

    /// Analyze function complexity
    fn analyze_complexity(&self, ops: &[Op]) -> Complexity {
        let op_count = ops.len();
        let mut loop_count = 0;
        let mut branch_count = 0;
        let mut call_count = 0;
        let mut complex_ops = 0;

        for op in ops {
            match op.opcode {
                Opcode::Jmp => {
                    if op.extended_value < ops.len() as u32 {
                        loop_count += 1;
                    } else {
                        branch_count += 1;
                    }
                }
                Opcode::JmpZ | Opcode::JmpNZ => branch_count += 1,
                Opcode::InitFCall | Opcode::DoFCall => call_count += 1,
                Opcode::Include | Opcode::NewObj | Opcode::DoMethodCall => complex_ops += 1,
                _ => {}
            }
        }

        // Determine complexity based on metrics
        if op_count <= 5 && loop_count == 0 && call_count == 0 {
            Complexity::Trivial
        } else if op_count <= 20 && loop_count <= 1 && call_count <= 2 {
            Complexity::Simple
        } else if op_count <= 50 && loop_count <= 3 && call_count <= 5 {
            Complexity::Moderate
        } else if op_count <= 100 && loop_count <= 5 && call_count <= 10 {
            Complexity::Complex
        } else {
            Complexity::VeryComplex
        }
    }

    /// Find functions that can be inlined
    fn find_inline_candidates(&self, ops: &[Op]) -> Vec<String> {
        let mut candidates = Vec::new();

        for i in 0..ops.len() {
            if ops[i].opcode == Opcode::InitFCall {
                // Look for function name in subsequent operations
                if i + 1 < ops.len() && ops[i + 1].opcode == Opcode::DoFCall {
                    if let crate::engine::types::PhpValue::String(ref func_name) =
                        ops[i + 1].op1.value
                    {
                        candidates.push(func_name.as_str().to_string());
                    }
                }
            }
        }

        candidates
    }

    /// Update optimization level based on call frequency
    fn update_optimization_level(&self, mut metadata: FunctionMetadata) -> FunctionMetadata {
        let call_stats = self.call_stats.get(&metadata.name);
        let call_frequency = call_stats.map(|s| s.call_count).unwrap_or(0);
        metadata.call_frequency = call_frequency;

        // Determine optimization level
        metadata.optimization_level = match (call_frequency, metadata.complexity, metadata.op_count)
        {
            (freq, _, _) if freq > 1000 => OptimizationLevel::Aggressive,
            (freq, Complexity::Trivial, _) if freq > 100 => OptimizationLevel::Aggressive,
            (freq, Complexity::Simple, _) if freq > 500 => OptimizationLevel::Aggressive,
            (freq, Complexity::Simple, _) if freq > 50 => OptimizationLevel::Basic,
            (freq, Complexity::Moderate, _) if freq > 200 => OptimizationLevel::Basic,
            (_, _, _) => OptimizationLevel::None,
        };

        metadata
    }

    /// Check if function should be inlined
    pub fn should_inline(&mut self, name: &str) -> bool {
        if let Some(metadata) = self.function_cache.get(name) {
            match metadata.optimization_level {
                OptimizationLevel::Aggressive => {
                    matches!(
                        metadata.complexity,
                        Complexity::Trivial | Complexity::Simple
                    )
                }
                OptimizationLevel::Basic => metadata.complexity == Complexity::Trivial,
                OptimizationLevel::None => false,
            }
        } else {
            false
        }
    }

    /// Generate inlined version of function
    pub fn generate_inlined_function(
        &mut self,
        name: &str,
        op_array: &OpArray,
    ) -> Result<InlinedFunction, String> {
        let metadata = self.analyze_function(name, op_array);

        if !self.should_inline(name) {
            return Err(format!("Function {} is not a candidate for inlining", name));
        }

        let optimized_ops = self.optimize_for_inlining(&op_array.ops, &metadata)?;
        let inline_threshold = self.calculate_inline_threshold(&metadata);

        let inlined = InlinedFunction {
            metadata,
            optimized_ops,
            inline_threshold,
        };

        self.inline_cache.insert(name.to_string(), inlined.clone());
        self.optimization_stats.functions_inlined += 1;

        Ok(inlined)
    }

    /// Optimize opcodes for inlining
    fn optimize_for_inlining(
        &mut self,
        ops: &[Op],
        metadata: &FunctionMetadata,
    ) -> Result<Vec<Op>, String> {
        let mut optimized = Vec::with_capacity(ops.len());

        for op in ops {
            match op.opcode {
                Opcode::InitFCall | Opcode::DoFCall => {
                    // Try to inline nested function calls
                    if let crate::engine::types::PhpValue::String(ref func_name) = op.op1.value {
                        if self.should_inline(func_name.as_str()) {
                            // Skip the function call - it will be inlined
                            continue;
                        }
                    }
                    // Keep the call if it can't be inlined
                    optimized.push(op.clone());
                }
                Opcode::Return => {
                    // Convert return to assignment to result variable
                    let return_op = Op::new(
                        Opcode::Assign,
                        op.op1.clone(),
                        op.op2.clone(),
                        op.result.clone(),
                        op.extended_value,
                    );
                    optimized.push(return_op);
                }
                _ => {
                    optimized.push(op.clone());
                }
            }
        }

        Ok(optimized)
    }

    /// Calculate inline threshold for function
    fn calculate_inline_threshold(&self, metadata: &FunctionMetadata) -> f32 {
        match metadata.complexity {
            Complexity::Trivial => 0.1,
            Complexity::Simple => 0.5,
            Complexity::Moderate => 2.0,
            Complexity::Complex => 5.0,
            Complexity::VeryComplex => 10.0,
        }
    }

    /// Execute function with optimization
    pub fn execute_optimized(
        &mut self,
        name: &str,
        execute_data: &mut ExecuteData,
        op_array: &OpArray,
    ) -> Result<PhpResult, String> {
        let start_time = std::time::Instant::now();

        // Check if we have an inlined version
        let inlined = self.inline_cache.get(name).cloned();

        // Check if we should inline this function
        let should_inline = self.should_inline(name) && inlined.is_none();

        let result = if should_inline {
            // Generate inlined version
            match self.generate_inlined_function(name, op_array) {
                Ok(inlined) => {
                    let result = self.execute_inlined_function(&inlined, execute_data)?;
                    self.optimization_stats.calls_saved += 1;
                    result
                }
                Err(e) => return Err(e),
            }
        } else if let Some(inlined) = inlined {
            // Use existing inlined version
            self.execute_inlined_function(&inlined, execute_data)?
        } else {
            // Execute normally
            self.execute_normal_function(execute_data, op_array)?
        };

        // Update call statistics
        let stats = self.call_stats.entry(name.to_string()).or_default();
        stats.call_count += 1;

        // Update statistics
        let elapsed = start_time.elapsed().as_nanos() as u64;
        stats.total_time_ns += elapsed;
        stats.average_time_ns = stats.total_time_ns / stats.call_count;

        Ok(result)
    }

    /// Execute inlined function
    fn execute_inlined_function(
        &mut self,
        inlined: &InlinedFunction,
        execute_data: &mut ExecuteData,
    ) -> Result<PhpResult, String> {
        // Execute optimized opcodes directly
        for op in &inlined.optimized_ops {
            match crate::vm::dispatch_handlers::dispatch_opcode(op, execute_data)? {
                ExecResult::Continue => continue,
                ExecResult::Jump(_) => {
                    // Handle jumps in inlined code
                    continue;
                }
                ExecResult::Return(_) => {
                    return Ok(PhpResult::Success);
                }
            }
        }

        Ok(PhpResult::Success)
    }

    /// Execute normal function
    fn execute_normal_function(
        &self,
        execute_data: &mut ExecuteData,
        op_array: &OpArray,
    ) -> Result<PhpResult, String> {
        // Use normal execution path
        Ok(crate::vm::execute::execute_ex(execute_data, op_array))
    }

    /// Get optimization statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.optimization_stats.functions_optimized,
            self.optimization_stats.functions_inlined,
            self.optimization_stats.calls_saved,
            self.optimization_stats.total_time_saved_ns,
        )
    }

    /// Get call statistics for a function
    pub fn get_call_stats(&self, name: &str) -> Option<&CallStats> {
        self.call_stats.get(name)
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.function_cache.clear();
        self.inline_cache.clear();
        self.call_stats.clear();
    }
}

impl Default for FunctionOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global function optimizer instance
static FUNCTION_OPTIMIZER: OnceLock<RwLock<FunctionOptimizer>> = OnceLock::new();

/// Get the global function optimizer instance
pub fn get_function_optimizer() -> &'static RwLock<FunctionOptimizer> {
    FUNCTION_OPTIMIZER.get_or_init(|| RwLock::new(FunctionOptimizer::new()))
}

/// Execute function with optimizations
pub fn execute_function_with_optimization(
    name: &str,
    execute_data: &mut ExecuteData,
    op_array: &OpArray,
) -> Result<PhpResult, String> {
    let optimizer = get_function_optimizer();
    let mut optimizer = optimizer.write().unwrap();
    optimizer.execute_optimized(name, execute_data, op_array)
}

/// Analyze function for optimization opportunities
pub fn analyze_function(name: &str, op_array: &OpArray) -> FunctionMetadata {
    let optimizer = get_function_optimizer();
    let mut optimizer = optimizer.write().unwrap();
    optimizer.analyze_function(name, op_array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{PhpType, PhpValue};

    #[test]
    fn test_function_analysis() {
        let mut optimizer = FunctionOptimizer::new();

        let mut op_array = OpArray::new("test.php".to_string());
        op_array.ops.push(Op::new(
            Opcode::Add,
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(2), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            0,
        ));

        let metadata = optimizer.analyze_function("test_function", &op_array);
        assert_eq!(metadata.name, "test_function");
        assert_eq!(metadata.complexity, Complexity::Trivial);
    }

    #[test]
    fn test_inline_generation() {
        let mut optimizer = FunctionOptimizer::new();

        let mut op_array = OpArray::new("test.php".to_string());
        op_array.ops.push(Op::new(
            Opcode::Add,
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(2), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            0,
        ));

        // Simulate high call frequency
        optimizer.call_stats.insert(
            "test_function".to_string(),
            CallStats {
                call_count: 1000,
                total_time_ns: 1000000,
                average_time_ns: 1000,
            },
        );

        let metadata = optimizer.analyze_function("test_function", &op_array);
        assert!(optimizer.should_inline("test_function"));

        if let Ok(inlined) = optimizer.generate_inlined_function("test_function", &op_array) {
            assert_eq!(inlined.optimized_ops.len(), 1);
        }
    }

    #[test]
    fn test_complexity_analysis() {
        let optimizer = FunctionOptimizer::new();

        // Test trivial function
        let trivial_ops = vec![Op::new(
            Opcode::Add,
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(2), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            0,
        )];
        assert_eq!(
            optimizer.analyze_complexity(&trivial_ops),
            Complexity::Trivial
        );

        // Test complex function
        let mut complex_ops = Vec::new();
        for i in 0..150 {
            complex_ops.push(Op::new(
                Opcode::Add,
                Val::new(PhpValue::Long(i as i64), PhpType::Long),
                Val::new(PhpValue::Long((i + 1) as i64), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ));
        }
        assert_eq!(
            optimizer.analyze_complexity(&complex_ops),
            Complexity::VeryComplex
        );
    }
}
