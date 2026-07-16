CREATE INDEX saved_profiles_group_bounded_list
    ON saved_profiles(group_name, favorite DESC, saved_order, profile_id);
