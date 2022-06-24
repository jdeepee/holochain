use crate::integrity::*;
use hdk::prelude::*;

#[hdk_extern(infallible)]
fn scheduled_fn(_: Option<Schedule>) -> Option<Schedule> {
    if HDK
        .with(|h| {
            h.borrow().create(CreateInput {
                input: EntryInput::App(AppEntry {
                    entry_def_index: EntryDefIndex::try_from(EntryTypesUnit::Tick)?,
                    visibility: EntryVisibility::Public,
                    entry: Tick.try_into().unwrap(),
                }),
                // This will be running concurrently with cron_scheduled_fn.
                chain_top_ordering: ChainTopOrdering::Relaxed,
            })
        })
        .is_err()
    {
        return Some(Schedule::Ephemeral(std::time::Duration::from_millis(1)));
    }
    if hdk::prelude::query(
        ChainQueryFilter::default().entry_type(EntryTypesUnit::Tick.try_into().unwrap()),
    )
    .unwrap()
    .len()
        < TICKS
    {
        Some(Schedule::Ephemeral(std::time::Duration::from_millis(1)))
    } else {
        None
    }
}

#[hdk_extern(infallible)]
fn cron_scheduled_fn(_: Option<Schedule>) -> Option<Schedule> {
    HDK.with(|h| {
        h.borrow().create(CreateInput::new(
            EntryInput::App(AppEntry {
                entry_def_index: EntryDefIndex::try_from(EntryTypesUnit::Tock)?,
                visibility: EntryVisibility::Public,
                entry: Tock.try_into().unwrap(),
            }),
            // This will be running concurrently with scheduled_fn.
            ChainTopOrdering::Relaxed,
        )?)
    })
    .ok();
    Some(Schedule::Persisted("* * * * * * *".to_string()))
}

#[hdk_extern]
fn schedule(_: ()) -> ExternResult<()> {
    hdk::prelude::schedule("scheduled_fn")?;
    hdk::prelude::schedule("cron_scheduled_fn")?;
    Ok(())
}

#[hdk_extern]
fn query_tick(_: ()) -> ExternResult<Vec<Record>> {
    hdk::prelude::query(
        ChainQueryFilter::default().entry_type(EntryTypesUnit::Tick.try_into().unwrap()),
    )
}

#[hdk_extern]
fn query_tock(_: ()) -> ExternResult<Vec<Record>> {
    hdk::prelude::query(
        ChainQueryFilter::default().entry_type(EntryTypesUnit::Tock.try_into().unwrap()),
    )
}
