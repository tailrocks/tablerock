CREATE INDEX saved_profiles_bounded_list
    ON saved_profiles(favorite DESC, saved_order, profile_id);
