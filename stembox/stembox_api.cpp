#include <stembox_api.h>

#include <taglib/mp4itemfactory.h>

namespace {

TagLib::ByteVector renderAtom(const TagLib::ByteVector &name,
                              const TagLib::ByteVector &data) {
  return TagLib::ByteVector::fromUInt(data.size() + 8) + name + data;
}

TagLib::ByteVector padUdta(const TagLib::ByteVector &data, int length = -1) {
  if (length == -1) {
    length = ((data.size() + 1023) & ~1023) - data.size();
  }
  return renderAtom("free", TagLib::ByteVector(length, '\1'));
}
} // namespace

File::File(std::string path)
    : m_file(std::make_unique<TagLib::MP4::File>(path.c_str())),
      m_atoms(std::make_unique<TagLib::MP4::Atoms>(m_file.get())) {
  TagLib::MP4::AtomList atomPath = m_atoms->path("moov", "udta", "stem");

  if (atomPath.size() == 3) {
    auto atom = atomPath.back();
    m_file->seek(atom->offset() + 8);
    m_rawData = m_file->readBlock(atom->length() - 8);
  }
}

void File::updateParents(const TagLib::MP4::AtomList &path,
                         TagLib::offset_t delta, int ignore) {
  if (static_cast<int>(path.size()) <= ignore)
    return;

  auto itEnd = path.end();
  std::advance(itEnd, 0 - ignore);

  for (auto it = path.begin(); it != itEnd; ++it) {
    m_file->seek((*it)->offset());
    long size = m_file->readBlock(4).toUInt();
    // 64-bit
    if (size == 1) {
      m_file->seek(4, TagLib::MP4::File::Current); // Skip name
      long long longSize = m_file->readBlock(8).toLongLong();
      // Seek the offset of the 64-bit size
      m_file->seek((*it)->offset() + 8);
      m_file->writeBlock(TagLib::ByteVector::fromLongLong(longSize + delta));
    }
    // 32-bit
    else {
      m_file->seek((*it)->offset());
      m_file->writeBlock(TagLib::ByteVector::fromUInt(
          static_cast<unsigned int>(size + delta)));
    }
  }
}

void File::updateOffsets(TagLib::offset_t delta, TagLib::offset_t offset) {
  if (TagLib::MP4::Atom *moov = m_atoms->find("moov")) {
    const TagLib::MP4::AtomList stco = moov->findall("stco", true);
    for (const auto &atom : stco) {
      if (atom->offset() > offset) {
        atom->addToOffset(delta);
      }
      m_file->seek(atom->offset() + 12);
      TagLib::ByteVector data = m_file->readBlock(atom->length() - 12);
      unsigned int count = data.toUInt();
      m_file->seek(atom->offset() + 16);
      unsigned int pos = 4;
      while (count--) {
        auto o = static_cast<TagLib::offset_t>(data.toUInt(pos));
        if (o > offset) {
          o += delta;
        }
        m_file->writeBlock(
            TagLib::ByteVector::fromUInt(static_cast<unsigned int>(o)));
        pos += 4;
      }
    }

    const TagLib::MP4::AtomList co64 = moov->findall("co64", true);
    for (const auto &atom : co64) {
      if (atom->offset() > offset) {
        atom->addToOffset(delta);
      }
      m_file->seek(atom->offset() + 12);
      TagLib::ByteVector data = m_file->readBlock(atom->length() - 12);
      unsigned int count = data.toUInt();
      m_file->seek(atom->offset() + 16);
      unsigned int pos = 4;
      while (count--) {
        long long o = data.toLongLong(pos);
        if (o > offset) {
          o += delta;
        }
        m_file->writeBlock(TagLib::ByteVector::fromLongLong(o));
        pos += 8;
      }
    }
  }

  if (TagLib::MP4::Atom *moof = m_atoms->find("moof")) {
    const TagLib::MP4::AtomList tfhd = moof->findall("tfhd", true);
    for (const auto &atom : tfhd) {
      if (atom->offset() > offset) {
        atom->addToOffset(delta);
      }
      m_file->seek(atom->offset() + 9);
      TagLib::ByteVector data = m_file->readBlock(atom->length() - 9);
      if (const unsigned int flags = data.toUInt(0, 3, true); flags & 1) {
        long long o = data.toLongLong(7U);
        if (o > offset) {
          o += delta;
        }
        m_file->seek(atom->offset() + 16);
        m_file->writeBlock(TagLib::ByteVector::fromLongLong(o));
      }
    }
  }
}

void File::saveExisting(const TagLib::MP4::AtomList &path) {
  auto data = renderAtom("stem", m_rawData);
  auto it = path.end();

  TagLib::MP4::Atom *udta = *(--it);
  TagLib::offset_t offset = udta->offset();
  TagLib::offset_t length = udta->length();

  TagLib::MP4::Atom *meta = *(--it);
  auto index = meta->children().cfind(udta);

  // check if there is an atom before 'udta', and possibly use it as padding
  if (index != meta->children().cbegin()) {
    auto prevIndex = std::prev(index);
    if (const TagLib::MP4::Atom *prev = *prevIndex; prev->name() == "free") {
      offset = prev->offset();
      length += prev->length();
    }
  }
  // check if there is an atom after 'udta', and possibly use it as padding
  auto nextIndex = std::next(index);
  if (nextIndex != meta->children().cend()) {
    if (const TagLib::MP4::Atom *next = *nextIndex; next->name() == "free") {
      length += next->length();
    }
  }

  TagLib::offset_t delta = data.size() - length;
  if (!data.isEmpty()) {
    if (delta > 0 || (delta < 0 && delta > -8)) {
      data.append(padUdta(data));
      delta = data.size() - length;
    } else if (delta < 0) {
      data.append(padUdta(data, static_cast<int>(-delta - 8)));
      delta = 0;
    }

    m_file->insert(data, offset, length);

    if (delta) {
      updateParents(path, delta, 1);
      updateOffsets(delta, offset);
    }
  } else {
    // Strip meta if data is empty, only the case when called from strip().
    if (TagLib::MP4::Atom *udta = *std::prev(it); udta->removeChild(meta)) {
      offset = meta->offset();
      delta = -meta->length();
      m_file->removeBlock(meta->offset(), meta->length());
      delete meta;

      if (delta) {
        updateParents(path, delta, 2);
        updateOffsets(delta, offset);
      }
    }
  }
}

void File::saveNew() {
  auto data = renderAtom("stem", m_rawData);

  TagLib::MP4::AtomList path = m_atoms->path("moov", "udta");
  if (path.size() != 2) {
    path = m_atoms->path("moov");
    data = renderAtom("udta", data);
  }

  TagLib::offset_t offset = path.back()->offset() + 8;
  m_file->insert(data, offset, 0);

  updateParents(path, data.size());
  updateOffsets(data.size(), offset);

  // Insert the newly created atoms into the tree to keep it up-to-date.

  m_file->seek(offset);
  path.back()->prependChild(new TagLib::MP4::Atom(m_file.get()));
}

bool File::save() {
  TagLib::MP4::AtomList atomPath = m_atoms->path("moov", "udta", "stem");
  if (atomPath.size() == 3) {
    saveExisting(atomPath);
  } else {
    saveNew();
  }
  return true;
}

std::string File::data() const {
  if (m_rawData.isEmpty()) {
    return "";
  } else {
    return std::string(m_rawData.data(), m_rawData.size());
  }
}
void File::setData(std::string data) {
  m_rawData.setData(data.c_str(), data.size());
}
