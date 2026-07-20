-- Voxtera Database Schema for Supabase
-- Run this in the Supabase SQL Editor

-- 1. User profiles (extends auth.users)
CREATE TABLE IF NOT EXISTS public.profiles (
    id UUID REFERENCES auth.users(id) PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    display_name TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 2. Game characters
CREATE TABLE IF NOT EXISTS public.characters (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID REFERENCES public.profiles(id) ON DELETE CASCADE NOT NULL,
    name TEXT NOT NULL,
    body_data JSONB NOT NULL DEFAULT '{}',
    stats_data JSONB NOT NULL DEFAULT '{}',
    skill_set_data JSONB NOT NULL DEFAULT '{}',
    inventory_data JSONB NOT NULL DEFAULT '{}',
    position_data JSONB NOT NULL DEFAULT '{}',
    waypoint_data JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Game settings per user
CREATE TABLE IF NOT EXISTS public.user_settings (
    user_id UUID REFERENCES public.profiles(id) PRIMARY KEY,
    settings_data JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 4. Auto-create profile on signup
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.profiles (id, username)
    VALUES (NEW.id, NEW.raw_user_meta_data->>'username');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Trigger to auto-create profile
CREATE OR REPLACE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();

-- 5. Row Level Security (RLS)
ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.characters ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_settings ENABLE ROW LEVEL SECURITY;

-- Profiles: users can read all, update only their own
CREATE POLICY "Profiles are viewable by everyone" ON public.profiles
    FOR SELECT USING (true);

CREATE POLICY "Users can update own profile" ON public.profiles
    FOR UPDATE USING (auth.uid() = id);

-- Characters: users can only access their own
CREATE POLICY "Users can view own characters" ON public.characters
    FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY "Users can insert own characters" ON public.characters
    FOR INSERT WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own characters" ON public.characters
    FOR UPDATE USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own characters" ON public.characters
    FOR DELETE USING (auth.uid() = user_id);

-- Settings: users can only access their own
CREATE POLICY "Users can view own settings" ON public.user_settings
    FOR SELECT USING (auth.uid() = user_id);

CREATE POLICY "Users can upsert own settings" ON public.user_settings
    FOR INSERT WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own settings" ON public.user_settings
    FOR UPDATE USING (auth.uid() = user_id);

-- 6. Indexes for performance
CREATE INDEX IF NOT EXISTS idx_characters_user_id ON public.characters(user_id);
CREATE INDEX IF NOT EXISTS idx_profiles_username ON public.profiles(username);
