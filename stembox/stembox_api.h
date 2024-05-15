#pragma once

#include <memory>
#include <string>

#include <taglib/mp4atom.h>
#include <taglib/mp4file.h>

class File {
public:
  File() {}
  File(std::string filename);
  bool save();

  std::string data() const;
  void setData(std::string);

private:
  void updateOffsets(TagLib::offset_t delta, TagLib::offset_t offset);
  void updateParents(const TagLib::MP4::AtomList &path, TagLib::offset_t delta,
                     int ignore = 0);
  void saveExisting(const TagLib::MP4::AtomList &path);
  void saveNew();

  std::unique_ptr<TagLib::MP4::File> m_file;
  std::unique_ptr<TagLib::MP4::Atoms> m_atoms;

  TagLib::ByteVector m_rawData;
};
