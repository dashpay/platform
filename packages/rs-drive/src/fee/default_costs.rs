// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Fee pool constants.
//!
//! This module defines constants related to fee distribution pools.
//!

/// Storage disk usage credit per byte
pub(crate) const STORAGE_DISK_USAGE_CREDIT_PER_BYTE: u64 = 27000;
/// Storage processing credit per byte
pub(crate) const STORAGE_PROCESSING_CREDIT_PER_BYTE: u64 = 400;
/// Storage load credit per byte
pub(crate) const STORAGE_LOAD_CREDIT_PER_BYTE: u64 = 400;
/// Non storage load credit per byte
pub(crate) const NON_STORAGE_LOAD_CREDIT_PER_BYTE: u64 = 30;
/// Query credit per byte
pub(crate) const QUERY_CREDIT_PER_BYTE: u64 = 10;
/// Storage seek cost
pub(crate) const STORAGE_SEEK_COST: u64 = 4000;
