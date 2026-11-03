// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Несколько команд на одном ПК: `teams_index.json` + `teams/{id}/…`.

mod index_io;
mod ops;
mod types;

pub use ops::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_stable() {
        let a = team_id_from_spreadsheet("abc");
        let b = team_id_from_spreadsheet("abc");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }
}
