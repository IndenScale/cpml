use chrono::NaiveDateTime;

use crate::model::activity::Activity;

/// A discrete time point at which the scene is evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyframe {
    pub date: NaiveDateTime,
    pub index: usize,
}

/// Extract all keyframes from the activities' timespan boundaries.
/// Collects start and end datetimes, deduplicates, and sorts chronologically.
pub fn extract_keyframes(activities: &[Activity]) -> Vec<Keyframe> {
    let mut dates: Vec<NaiveDateTime> = Vec::new();
    for a in activities {
        dates.push(a.timespan.start);
        dates.push(a.timespan.end);
    }
    dates.sort();
    dates.dedup();

    dates
        .into_iter()
        .enumerate()
        .map(|(i, date)| Keyframe { date, index: i })
        .collect()
}

/// Return the subset of activities active at the given keyframe.
/// An activity is active if start <= keyframe_date < end.
pub fn active_activities_at<'a>(
    keyframe: &Keyframe,
    activities: &'a [Activity],
) -> Vec<&'a Activity> {
    activities
        .iter()
        .filter(|a| a.timespan.contains(keyframe.date))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::activity::Timespan;
    use chrono::NaiveDate;

    fn dt(s: &str) -> NaiveDateTime {
        let d = NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        d.and_hms_opt(0, 0, 0).unwrap()
    }

    fn make_activity(id: &str, start: &str, end: &str) -> Activity {
        Activity {
            id: id.into(),
            name: None,
            series: None,
            timespan: Timespan {
                start: dt(start),
                end: dt(end),
            },
            geometry: None,
            probes: vec![],
            projections: vec![],
            depends_on: vec![],
        }
    }

    #[test]
    fn test_extract_keyframes() {
        let activities = vec![
            make_activity("A", "2026-01-01", "2026-01-10"),
            make_activity("B", "2026-01-05", "2026-01-15"),
        ];
        let kfs = extract_keyframes(&activities);
        assert_eq!(kfs.len(), 4);
        assert_eq!(kfs[0].date, dt("2026-01-01"));
        assert_eq!(kfs[1].date, dt("2026-01-05"));
        assert_eq!(kfs[2].date, dt("2026-01-10"));
        assert_eq!(kfs[3].date, dt("2026-01-15"));
    }

    #[test]
    fn test_active_activities_half_open() {
        let a = make_activity("A", "2026-01-01", "2026-01-10");
        let activities = vec![a];
        let kfs = extract_keyframes(&activities);

        // At start date: active
        let active = active_activities_at(&kfs[0], &activities);
        assert_eq!(active.len(), 1);

        // At end date: NOT active (half-open)
        let active = active_activities_at(&kfs[1], &activities);
        assert_eq!(active.len(), 0);
    }
}
