use wmidi;

pub fn go() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(31).unwrap()
    )
}

pub fn clear() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(30).unwrap()
    )
}

pub fn stop() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(29).unwrap()
    )
}

pub fn undo() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(28).unwrap()
    )
}

pub fn scene1() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn scene2() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(1).unwrap()
    )
}

pub fn scene3() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(2).unwrap()
    )
}

pub fn scene4() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(3).unwrap()
    )
}
pub fn scene5() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(4).unwrap()
    )
}
pub fn scene6() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(5).unwrap()
    )
}
pub fn scene7() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(6).unwrap()
    )
}
pub fn scene8() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::ProgramChange(
        wmidi::Channel::Ch1,
    	wmidi::U7::try_from(7).unwrap()
    )
}

pub fn track1() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Ab5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track2() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::A5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track3() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Bb5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track4() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::B5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track5() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::E5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track6() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::F5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track7() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Gb5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track8() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::G5,
	wmidi::U7::try_from(0).unwrap()
    )
}


pub fn track9() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::C5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track10() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Db5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track11() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::D5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track12() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Eb5,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track13() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Ab4,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track14() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::A4,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track15() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::Bb4,
	wmidi::U7::try_from(0).unwrap()
    )
}

pub fn track16() -> wmidi::MidiMessage<'static> {
    wmidi::MidiMessage::NoteOff(
	wmidi::Channel::Ch10,
	wmidi::Note::B4,
	wmidi::U7::try_from(0).unwrap()
    )
}
