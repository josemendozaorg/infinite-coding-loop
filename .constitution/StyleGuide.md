# Style Guide & Coding Standards

**Language**: Rust (Edition 2021)
**Framework**: DASS Engine

## 1. Safety & Error Handling

### 1.1. No Panics
*   **Rule**: `unwrap()` and `expect()` are **FORBIDDEN** in production code.
*   **Exception**: Allowed in `#[test]` functions or integration test binaries.
*   **Replacement**: Use `anyhow::Result` for apps, `thiserror` for libs, and `?` propagation.

### 1.2. Strong Typing
*   **Rule**: Avoid `String` typing for domain concepts. Use "New Type" pattern.
*   **Bad**: `fn process_user(id: String)`
*   **Good**: `fn process_user(id: UserId)`

## 2. Documentation

### 2.1. Public Interfaces
*   **Rule**: All `pub` functions, structs, and enums must have a docstring (`///`).
*   **Format**:
    ```rust
    /// Short summary of what this does.
    ///
    /// # Arguments
    /// * `arg` - Description.
    ///
    /// # Returns
    /// Description of result.
    pub fn my_func() {}
    ```

## 3. Asynchronous Code

### 3.1. Cancellation Safety
*   **Rule**: All `async` functions must be cancellation-safe. Avoid holding `MutexGuard` across await points.
*   **Recommendation**: Use `tokio::sync::Mutex` if spanning awaits, or standard `std::sync::Mutex` for strictly synchronous critical sections.

## 4. Testing

### 4.1. Unit vs Property
*   **Rule**: Prefer `proptest` for logic with wide input ranges.
*   **Rule**: Use `#[test]` for specific regression cases.
