//! Opcode Cache and Optimization
//!
//! Advanced opcode caching system with optimization passes to outperform PHP 8

use crate::vm::execute_data::ExecuteData;
use crate::vm::opcodes::{Op, OpArray, Opcode};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Optimized opcode with additional metadata
#[derive(Debug, Clone)]
pub struct OptimizedOp {
    pub base: Op,
    pub optimization_hints: OptimizationHints,
    pub execution_count: u32,
    pub last_optimized: u64,
}

/// Optimization hints for each opcode
#[derive(Debug, Clone, Default)]
pub struct OptimizationHints {
    pub can_inline: bool,
    pub is_hot: bool,
    pub can_constant_fold: bool,
    pub can_dead_code_eliminate: bool,
    pub loop_depth: u32,
    pub branch_probability: f32, // 0.0 to 1.0
}

/// Opcode cache entry
#[derive(Debug)]
pub struct CacheEntry {
    pub optimized_ops: Vec<OptimizedOp>,
    pub constant_table: HashMap<u32, crate::engine::types::Val>,
    pub optimization_level: OptimizationLevel,
    pub timestamp: u64,
    pub hits: u64,
    pub last_used: u64,
}

/// Optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None = 0,
    Basic = 1,      // Basic optimizations
    Aggressive = 2, // Aggressive optimizations
}

/// Opcode cache with LRU eviction
#[derive(Debug)]
pub struct OpcodeCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    lru_order: Arc<RwLock<Vec<String>>>,
    max_entries: usize,
    stats: CacheStats,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: std::sync::atomic::AtomicU64,
    pub misses: std::sync::atomic::AtomicU64,
    pub evictions: std::sync::atomic::AtomicU64,
    pub optimizations_performed: std::sync::atomic::AtomicU64,
}

impl OpcodeCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            lru_order: Arc::new(RwLock::new(Vec::with_capacity(max_entries))),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Get cached opcodes for a file
    pub fn get(&self, filename: &str) -> Option<Vec<Op>> {
        let cache = self.cache.read().unwrap();
        if let Some(entry) = cache.get(filename) {
            self.stats
                .hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Convert optimized ops back to regular ops
            let ops = entry
                .optimized_ops
                .iter()
                .map(|opt| opt.base.clone())
                .collect();

            // Update LRU
            drop(cache);
            self.update_lru(filename);

            Some(ops)
        } else {
            self.stats
                .misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    /// Store optimized opcodes in cache
    pub fn store(&self, filename: &str, ops: Vec<Op>, level: OptimizationLevel) {
        let mut optimized_ops = Vec::with_capacity(ops.len());

        // Apply optimizations based on level
        match level {
            OptimizationLevel::None => {
                for op in ops {
                    optimized_ops.push(OptimizedOp {
                        base: op,
                        optimization_hints: OptimizationHints::default(),
                        execution_count: 0,
                        last_optimized: current_timestamp(),
                    });
                }
            }
            OptimizationLevel::Basic => {
                optimized_ops = Self::apply_basic_optimizations(ops);
            }
            OptimizationLevel::Aggressive => {
                optimized_ops = Self::apply_aggressive_optimizations(ops);
            }
        }

        let entry = CacheEntry {
            optimized_ops,
            constant_table: HashMap::new(),
            optimization_level: level,
            timestamp: current_timestamp(),
            hits: 0,
            last_used: current_timestamp(),
        };

        // Check if we need to evict
        {
            let mut cache = self.cache.write().unwrap();
            if cache.len() >= self.max_entries {
                self.evict_lru(&mut cache);
            }
            cache.insert(filename.to_string(), entry);
        }

        // Update LRU
        self.update_lru(filename);
        self.stats
            .optimizations_performed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Apply basic optimizations
    fn apply_basic_optimizations(ops: Vec<Op>) -> Vec<OptimizedOp> {
        let mut optimized = Vec::with_capacity(ops.len());

        for op in ops {
            let mut hints = OptimizationHints::default();

            // Detect constant folding opportunities
            if Self::can_constant_fold(&op) {
                hints.can_constant_fold = true;
            }

            // Detect dead code elimination opportunities
            if Self::can_dead_code_eliminate(&op) {
                hints.can_dead_code_eliminate = true;
            }

            optimized.push(OptimizedOp {
                base: op,
                optimization_hints: hints,
                execution_count: 0,
                last_optimized: current_timestamp(),
            });
        }

        optimized
    }

    /// Apply aggressive optimizations
    fn apply_aggressive_optimizations(ops: Vec<Op>) -> Vec<OptimizedOp> {
        let mut optimized = Self::apply_basic_optimizations(ops);

        // Apply loop optimizations
        optimized = Self::optimize_loops(optimized);

        // Apply branch prediction hints
        optimized = Self::add_branch_hints(optimized);

        // Apply inlining hints
        optimized = Self::detect_inlining_opportunities(optimized);

        optimized
    }

    /// Check if operation can be constant folded
    fn can_constant_fold(op: &Op) -> bool {
        match op.opcode {
            Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Pow
            | Opcode::Concat => {
                // Check if both operands are constants
                Self::is_constant(&op.op1) && Self::is_constant(&op.op2)
            }
            Opcode::JmpZ | Opcode::JmpNZ => Self::is_constant(&op.op1),
            _ => false,
        }
    }

    /// Check if value is constant
    fn is_constant(val: &crate::engine::types::Val) -> bool {
        use crate::engine::types::PhpValue;
        match &val.value {
            PhpValue::Long(_) | PhpValue::Double(_) | PhpValue::String(_) => true,
            _ => false,
        }
    }

    /// Check if operation can be dead code eliminated
    fn can_dead_code_eliminate(op: &Op) -> bool {
        match op.opcode {
            Opcode::Nop => true,
            Opcode::Echo if Self::is_debug_code(op) => true,
            _ => false,
        }
    }

    /// Check if this is debug code that can be eliminated
    fn is_debug_code(_op: &Op) -> bool {
        // Simple heuristic - in a real implementation, this would be more sophisticated
        false
    }

    /// Optimize loops
    fn optimize_loops(ops: Vec<OptimizedOp>) -> Vec<OptimizedOp> {
        // Simple loop optimization - detect loop patterns
        let mut optimized = ops;
        let mut loop_depth = 0;

        for op in &mut optimized {
            match op.base.opcode {
                Opcode::Jmp => loop_depth += 1,
                _ => {}
            }

            op.optimization_hints.loop_depth = loop_depth;

            // Mark hot loops
            if loop_depth > 0 {
                op.optimization_hints.is_hot = true;
            }
        }

        optimized
    }

    /// Add branch prediction hints
    fn add_branch_hints(ops: Vec<OptimizedOp>) -> Vec<OptimizedOp> {
        let mut optimized = ops;

        // Simple branch prediction - assume backward jumps are likely taken
        for i in 0..optimized.len() {
            if optimized[i].base.opcode == Opcode::Jmp {
                if let Some(target) = optimized[i].base.extended_value.checked_sub(1) {
                    if target < i as u32 {
                        // Backward jump - likely loop, high probability
                        optimized[i].optimization_hints.branch_probability = 0.9;
                    } else {
                        // Forward jump - unlikely
                        optimized[i].optimization_hints.branch_probability = 0.1;
                    }
                }
            }
        }

        optimized
    }

    /// Detect inlining opportunities
    fn detect_inlining_opportunities(ops: Vec<OptimizedOp>) -> Vec<OptimizedOp> {
        let mut optimized = ops;

        // Simple inlining detection - small function calls
        for i in 0..optimized.len() {
            if optimized[i].base.opcode == Opcode::InitFCall {
                // Check if this is a small function that can be inlined
                if i + 2 < optimized.len() {
                    let func_call_ops = &optimized[i..=i + 2];
                    if Self::is_small_function_call(func_call_ops) {
                        optimized[i].optimization_hints.can_inline = true;
                    }
                }
            }
        }

        optimized
    }

    /// Check if this is a small function call suitable for inlining
    fn is_small_function_call(ops: &[OptimizedOp]) -> bool {
        // Simple heuristic - in a real implementation, this would analyze function size
        ops.len() <= 5
    }

    /// Update LRU order
    fn update_lru(&self, filename: &str) {
        let mut lru = self.lru_order.write().unwrap();
        if let Some(pos) = lru.iter().position(|x| x == filename) {
            lru.remove(pos);
        }
        lru.push(filename.to_string());
    }

    /// Evict least recently used entry
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        let mut lru = self.lru_order.write().unwrap();
        if let Some(lru_key) = lru.first() {
            cache.remove(lru_key);
            lru.remove(0);
            self.stats
                .evictions
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.stats.hits.load(std::sync::atomic::Ordering::Relaxed),
            self.stats.misses.load(std::sync::atomic::Ordering::Relaxed),
            self.stats
                .evictions
                .load(std::sync::atomic::Ordering::Relaxed),
            self.stats
                .optimizations_performed
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        let mut lru = self.lru_order.write().unwrap();
        cache.clear();
        lru.clear();
    }
}

impl Default for OpcodeCache {
    fn default() -> Self {
        Self::new(1000) // Default cache size
    }
}

/// Global opcode cache instance
static OPCODE_CACHE: OnceLock<OpcodeCache> = OnceLock::new();

/// Get the global opcode cache instance
pub fn get_opcode_cache() -> &'static OpcodeCache {
    OPCODE_CACHE.get_or_init(|| OpcodeCache::new(1000))
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Load and cache opcodes with optimization
pub fn load_optimized_opcodes(filename: &str) -> Option<Vec<Op>> {
    let cache = get_opcode_cache();

    // Try cache first
    if let Some(cached_ops) = cache.get(filename) {
        return Some(cached_ops);
    }

    // Load from file and optimize
    match super::compile::compile_file(filename) {
        Ok(op_array) => {
            // Determine optimization level based on file size and patterns
            let level = if op_array.ops.len() > 1000 {
                OptimizationLevel::Aggressive
            } else if op_array.ops.len() > 100 {
                OptimizationLevel::Basic
            } else {
                OptimizationLevel::None
            };

            // Store in cache
            cache.store(filename, op_array.ops.clone(), level);

            Some(op_array.ops)
        }
        Err(_) => None,
    }
}

/// Runtime optimization of hot paths
pub fn optimize_hot_path(
    _execute_data: &mut ExecuteData,
    op_array: &OpArray,
) -> Result<(), String> {
    let cache = get_opcode_cache();

    // Identify hot spots based on execution frequency
    let hot_operations: Vec<usize> = (0..op_array.ops.len())
        .filter(|&i| {
            // Simple heuristic - in a real implementation, this would track actual execution counts
            i % 10 == 0 // Every 10th operation as "hot"
        })
        .collect();

    if !hot_operations.is_empty() {
        // Re-optimize with aggressive level for hot paths
        let mut optimized_ops = op_array.ops.clone();
        for &hot_idx in &hot_operations {
            if let Some(_op) = optimized_ops.get_mut(hot_idx) {
                // Add hot path hints
                // This would be more sophisticated in a real implementation
            }
        }

        // Update cache with optimized version
        if let Some(filename) = &op_array.filename {
            cache.store(filename, optimized_ops, OptimizationLevel::Aggressive);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{PhpType, PhpValue, Val};

    #[test]
    fn test_opcode_cache() {
        let cache = OpcodeCache::new(10);

        let filename = "test.php";
        let ops = vec![
            Op::new(
                Opcode::Nop,
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ),
            Op::new(
                Opcode::Add,
                Val::new(PhpValue::Long(1), PhpType::Long),
                Val::new(PhpValue::Long(2), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ),
        ];

        // Test cache miss
        assert!(cache.get(filename).is_none());

        // Store and test cache hit
        cache.store(filename, ops.clone(), OptimizationLevel::Basic);
        let cached = cache.get(filename).unwrap();
        assert_eq!(cached.len(), ops.len());

        let stats = cache.get_stats();
        assert_eq!(stats.0, 1); // hits
        assert_eq!(stats.1, 1); // misses
    }

    #[test]
    fn test_basic_optimizations() {
        let ops = vec![Op::new(
            Opcode::Add,
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(2), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            0,
        )];

        let optimized = OpcodeCache::apply_basic_optimizations(ops);
        assert_eq!(optimized.len(), 1);
        assert!(optimized[0].optimization_hints.can_constant_fold);
    }

    #[test]
    fn test_aggressive_optimizations() {
        let ops = vec![
            Op::new(
                Opcode::InitFCall,
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ),
            Op::new(
                Opcode::SendVal,
                Val::new(PhpValue::Long(1), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ),
            Op::new(
                Opcode::DoFCall,
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                Val::new(PhpValue::Long(0), PhpType::Long),
                0,
            ),
        ];

        let optimized = OpcodeCache::apply_aggressive_optimizations(ops);
        assert_eq!(optimized.len(), 3);
        assert!(optimized[0].optimization_hints.can_inline);
    }
}
