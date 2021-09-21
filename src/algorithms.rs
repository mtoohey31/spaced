use chrono::{Date, Duration, Utc};

pub fn leitner(review_history: &mut Vec<(Date<Utc>, bool)>) -> bool {
    let last_review = match review_history.pop() {
        Some(l) => l,
        None => return true,
    };

    review_history.push(last_review);

    if last_review.1 {
        let mut spacing = 0.5_f64;
        let mut prev_date = chrono::MIN_DATE;
        let mut failure_registered_today = false;

        for event in review_history {
            if event.0 == prev_date {
                if !failure_registered_today && !event.1 {
                    spacing *= 0.25;
                    failure_registered_today = true;
                }
            } else {
                prev_date = event.0;

                if event.1 {
                    spacing *= 2.0;
                    failure_registered_today = false;
                } else {
                    spacing *= 0.5;
                    failure_registered_today = true;
                }
            }
        }

        last_review.0 + Duration::days((spacing.round() as i64).max(1)) <= Utc::today()
    } else {
        true
    }
}
