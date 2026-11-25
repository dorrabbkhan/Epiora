//! `SET <clause>` (spec §5.4): writes the view's display/process
//! options; `materialize` and the FREQ/LIST paths read them.

use crate::manage::SetOption;
use crate::view::WorkingView;
use crate::LangError;

pub(crate) fn set(view: &mut WorkingView, o: &SetOption) -> Result<(), LangError> {
    match o {
        SetOption::Process(p) => view.settings.process = *p,
        SetOption::Missing(on) => view.settings.include_missing = *on,
        SetOption::Deleted(on) => view.settings.deleted = *on,
        SetOption::Statistics(s) => view.settings.statistics = *s,
        SetOption::Percents(on) => view.settings.percents = *on,
        SetOption::Ignore(on) => view.settings.ignore = *on,
        SetOption::Select(on) => view.settings.select = *on,
    }
    Ok(())
}
