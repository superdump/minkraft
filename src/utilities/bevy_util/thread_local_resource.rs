/*
 * MIT License
 *
 * Copyright (c) 2020 bonsairobo
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 */

use std::sync::Arc;
use thread_local::ThreadLocal;

pub struct ThreadLocalResource<T>
where
    T: Send,
{
    tls: Arc<ThreadLocal<T>>,
}

impl<T> Default for ThreadLocalResource<T>
where
    T: Send,
{
    fn default() -> Self {
        Self {
            tls: Default::default(),
        }
    }
}

impl<T> ThreadLocalResource<T>
where
    T: Send,
{
    pub fn new() -> Self {
        Self {
            tls: Arc::new(ThreadLocal::new()),
        }
    }

    pub fn get(&self) -> ThreadLocalResourceHandle<T> {
        ThreadLocalResourceHandle {
            tls: self.tls.clone(),
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = T> {
        let tls = match Arc::try_unwrap(self.tls) {
            Ok(x) => x,
            Err(_) => panic!(
                "Failed to unwrap Arc'd thread-local storage; \
                there must be an outstanding strong reference"
            ),
        };

        tls.into_iter()
    }
}

pub struct ThreadLocalResourceHandle<T>
where
    T: Send,
{
    tls: Arc<ThreadLocal<T>>,
}

impl<T> ThreadLocalResourceHandle<T>
where
    T: Send,
{
    pub fn get_or_create_with(&self, create: impl FnOnce() -> T) -> &T {
        self.tls.get_or(create)
    }

    pub fn get_or_default(&self) -> &T
    where
        T: Default,
    {
        self.tls.get_or_default()
    }
}
