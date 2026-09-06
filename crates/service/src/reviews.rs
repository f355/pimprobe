// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use pimprobe_core::{RoutinePlan, RoutineResult};

#[derive(Default)]
pub struct Reviews {
    pending: Option<(String, u64, RoutinePlan)>,
    completed: Option<(String, u64, RoutinePlan, RoutineResult)>,
}

impl Reviews {
    pub fn put(&mut self, plan: RoutinePlan, session: u64) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.pending = Some((id.clone(), session, plan));
        self.completed = None;
        id
    }

    pub fn take(&mut self, id: &str, session: u64) -> Option<RoutinePlan> {
        if self
            .pending
            .as_ref()
            .is_some_and(|(token, epoch, _)| token == id && *epoch == session)
        {
            self.pending.take().map(|(_, _, plan)| plan)
        } else {
            None
        }
    }

    pub fn finish(&mut self, id: String, session: u64, plan: RoutinePlan, result: RoutineResult) {
        if result.settled {
            self.completed = Some((id, session, plan, result));
        }
    }

    pub fn take_completed(
        &mut self,
        id: &str,
        session: u64,
    ) -> Option<(RoutinePlan, RoutineResult)> {
        if self
            .completed
            .as_ref()
            .is_some_and(|(token, epoch, _, _)| token == id && *epoch == session)
        {
            self.completed
                .take()
                .map(|(_, _, plan, result)| (plan, result))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pimprobe_core::{MockController, RoutineConfig, review};

    #[test]
    fn authorizations_are_one_shot_and_bound_to_connection() {
        let controller = MockController::new();
        controller.set_extended(true).unwrap();
        let plan = review(controller.state(), RoutineConfig::default()).unwrap();
        let mut reviews = Reviews::default();
        let id = reviews.put(plan.clone(), 1);
        assert!(reviews.take(&id, 2).is_none());
        assert!(reviews.take(&id, 1).is_some());
        assert!(reviews.take(&id, 1).is_none());
        reviews.finish(
            id.clone(),
            1,
            plan,
            RoutineResult {
                settled: true,
                ..Default::default()
            },
        );
        assert!(reviews.take_completed(&id, 2).is_none());
        assert!(reviews.take_completed(&id, 1).is_some());
        assert!(reviews.take_completed(&id, 1).is_none());
    }
}
