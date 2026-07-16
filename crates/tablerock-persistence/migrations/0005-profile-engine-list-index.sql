CREATE INDEX saved_profiles_engine_bounded_list
    ON saved_profiles(engine, favorite DESC, saved_order, profile_id);
