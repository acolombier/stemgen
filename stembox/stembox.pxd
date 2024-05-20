from libcpp.string cimport string
from libcpp cimport bool

cdef extern from 'stembox_api.h':
    cdef cppclass File:
        File() except +
        File(string) except +
        string data()
        void setData(string)
        bool save()
